use crate::{model::*, store::Store};
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};

pub fn refresh(force: bool) -> Result<Vec<String>> {
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
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("Subbit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let mut updates = BTreeMap::new();
    let mut errors = vec![];
    if fiat {
        let result = (|| -> Result<()> {
            let rows: Vec<Value> = client
                .get("https://api.frankfurter.dev/v2/rates?base=USD")
                .send()
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.json())
                .map_err(|e| e.to_string())?;
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
            let data: Value = client
                .get("https://api.coinbase.com/v2/exchange-rates?currency=USD")
                .send()
                .and_then(|r| r.error_for_status())
                .and_then(|r| r.json())
                .map_err(|e| e.to_string())?;
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
        store.update(|db| {
            db.rates.extend(updates);
            Ok(())
        })?;
    }
    Ok(errors)
}
