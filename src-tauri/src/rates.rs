use crate::{model::*, store::Store};
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::Value;
use std::{collections::BTreeMap};

pub fn refresh(force: bool) -> Result<Vec<String>> {
    crate::network::require_online()?;
    let store = Store::system()?;
    let (db, _) = store.read()?;
    if !force && !db.settings.auto_rates {
        return Ok(vec![]);
    }
    let now = Utc::now();
    let stale = |code: &str, seconds: i64| {
        db.rates
            .get(code)
            .and_then(|r| chrono::DateTime::parse_from_rfc3339(&r.as_of).ok())
            .is_none_or(|t| now.signed_duration_since(t).num_seconds() >= seconds)
    };
    let fiat = force
        || CURRENCIES[..26]
            .iter()
            .filter(|c| **c != "USD")
            .any(|c| stale(c, 86400));
    let crypto = force
        || db
            .subscriptions
            .iter()
            .any(|s| CURRENCIES[26..].contains(&s.currency.as_str()) && stale(&s.currency, 900));
    if !fiat && !crypto {
        return Ok(vec![]);
    }
    let mut updates = BTreeMap::new();
    let mut errors = vec![];
    if fiat {
        let result = (|| -> Result<()> {
            crate::network::require_online()?;
            let bytes = crate::network::download("https://api.frankfurter.dev/v2/rates?base=USD", 1_000_000)?.bytes;
            let rows: Vec<Value> = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            for row in rows {
                if let Some(code) = row["quote"].as_str() {
                    if CURRENCIES[..26].contains(&code) {
                        let quote = decimal(&row["rate"].to_string())?;
                        if quote > Decimal::ZERO {
                            updates.insert(
                                code.to_string(),
                                Rate {
                                    usd: (Decimal::ONE / quote).round_dp(12).to_string(),
                                    as_of: now.to_rfc3339(),
                                },
                            );
                        }
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            errors.push(format!("Fiat: {e}"));
        }
    }
    if crypto {
        let result = (|| -> Result<()> {
            crate::network::require_online()?;
            let bytes = crate::network::download("https://api.coinbase.com/v2/exchange-rates?currency=USD", 1_000_000)?.bytes;
            let data: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            for code in &CURRENCIES[26..] {
                if let Some(raw) = data["data"]["rates"][*code].as_str() {
                    let rate: Decimal = raw.parse().map_err(|_| "Invalid quote")?;
                    if rate > Decimal::ZERO {
                        updates.insert(
                            code.to_string(),
                            Rate {
                                usd: (Decimal::ONE / rate).round_dp(12).to_string(),
                                as_of: now.to_rfc3339(),
                            },
                        );
                    }
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            errors.push(format!("Crypto: {e}"));
        }
    }
    if !updates.is_empty() {
        crate::network::require_online()?;
        store.update(|db| {
            crate::network::require_online()?;
            db.rates.extend(updates);
            Ok(())
        })?;
    }
    Ok(errors)
}

/// Dated fiat snapshot is a fallback only; never overwrite an existing cache or fixed rate.
pub fn bundled() -> &'static BTreeMap<String, Rate> {
    static RATES: std::sync::OnceLock<BTreeMap<String, Rate>> = std::sync::OnceLock::new();
    RATES.get_or_init(|| {
        let value: Value = serde_json::from_str(include_str!("../bundled-rates.json")).expect("bundled rates");
        serde_json::from_value(value["rates"].clone()).expect("bundled rate entries")
    })
}
pub fn effective(db: &Database) -> BTreeMap<String, Rate> {
    let mut rates = bundled().clone();
    rates.extend(db.rates.clone());
    rates
}

pub fn validate_import(rates: &BTreeMap<String, Rate>) -> Result<()> {
    if rates.is_empty() || rates.len() > CURRENCIES.len() { return Err("Provide supported exchange rates".into()); }
    for (code, rate) in rates {
        if !CURRENCIES.contains(&code.as_str()) || decimal(&rate.usd)? <= Decimal::ZERO
            || (code == "USD" && decimal(&rate.usd)? != Decimal::ONE)
            || chrono::DateTime::parse_from_rfc3339(&rate.as_of).is_err() {
            return Err("Rates require a supported currency, positive USD quote and RFC3339 timestamp".into());
        }
    }
    Ok(())
}
pub fn import(store: &Store, id: String, revision: u64, rates: BTreeMap<String, Rate>) -> Result<Value> {
    validate_import(&rates)?;
    if id.is_empty() || id.len() > 100 { return Err("A stable importId is required".into()); }
    store.update_if_changed(true, |db| {
        if !db.settings.mcp_enabled { return Err("MCP is disabled".into()); }
        if let Some(previous) = db.rate_imports.get(&id) {
            if previous != &rates { return Err("importId was used for different rates".into()); }
            return Ok((serde_json::json!({"replayed":true,"updated":0}), false));
        }
        if db.revision != revision { return Err("Data changed; preview rates again".into()); }
        db.rates.extend(rates.clone());
        let count = rates.len();
        db.rate_imports.insert(id, rates);
        Ok((serde_json::json!({"updated":count,"replayed":false}), true))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_import_preserves_assets_overrides_and_rejects_stale_or_reused_ids() {
        let root = tempfile::tempdir().unwrap();
        let store = Store::at(root.path().into());
        store.update(|db| { db.settings.mcp_enabled = true; db.settings.fixed_rates.insert("CNY".into(), "0.2".into()); Ok(()) }).unwrap();
        let revision = store.read().unwrap().0.revision;
        let rates = BTreeMap::from([("CNY".into(), Rate { usd: "0.14".into(), as_of: "2026-10-17T12:00:00Z".into() })]);
        import(&store, "one".into(), revision, rates.clone()).unwrap();
        assert_eq!(store.read().unwrap().0.usd_rate("CNY").unwrap().to_string(), "0.2");
        let (before, path) = store.read().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let backup = std::fs::read(path.with_extension("json.bak")).unwrap();
        assert_eq!(import(&store, "one".into(), revision, rates.clone()).unwrap()["replayed"], true);
        assert_eq!(store.read().unwrap().0.revision, before.revision);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(std::fs::read(path.with_extension("json.bak")).unwrap(), backup);
        assert!(import(&store, "two".into(), revision, rates.clone()).is_err());
        let mut changed = rates; changed.get_mut("CNY").unwrap().usd = "0.15".into();
        assert!(import(&store, "one".into(), revision, changed).is_err());
        assert!(!bundled().is_empty());
        assert_eq!(Database::default().usd_rate("BTC"), None);
    }
}
