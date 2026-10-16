use rust_decimal::Decimal;
use std::str::FromStr;
use subbit_lib::{
    model::*,
    store::{write_json, Store},
};

fn sample() -> Subscription {
    serde_json::from_value(serde_json::json!({"id":"one","name":"Example","serviceId":null,"category":"other","plan":"Pro","icon":null,"amount":"1200","currency":"CNY","kind":"recurring","cycleUnit":"year","cycleCount":1,"purchasedAt":"2024-01-31","expiresAt":null,"autoRenew":true,"active":true,"reimbursement":"fixed","reimbursementValue":"600","paymentMethod":"apple","notes":"","fixedUsdRate":null})).unwrap()
}
fn day(s: &str) -> chrono::NaiveDate {
    date(s).unwrap()
}
fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("subscriptions.json");
    write_json(&path, &Database::default(), false).unwrap();
    write_json(&dir.path().join("location.json"), &path, false).unwrap();
    let store = Store::at(dir.path().into());
    (dir, store)
}

#[test]
fn yearly_reimbursement_and_hourly_cost() {
    let mut db = Database::default();
    db.subscriptions.push(sample());
    let result = db.summary("CNY", "month", day("2026-10-16")).unwrap();
    assert_eq!(result["bill"], "100");
    assert_eq!(result["actual"], "50");
    let hour = db.summary("CNY", "hour", day("2026-10-16")).unwrap();
    assert_eq!(
        Decimal::from_str(hour["actual"].as_str().unwrap())
            .unwrap()
            .round_dp(4)
            .to_string(),
        "0.0685"
    );
}
#[test]
fn renewal_preserves_original_month_end() {
    let mut s = sample();
    s.cycle_unit = "month".into();
    s.expires_at = Some("2025-01-31".into());
    assert_eq!(
        s.due_date(day("2025-02-01")).unwrap(),
        Some(day("2025-02-28"))
    );
    assert_eq!(
        s.due_date(day("2025-03-01")).unwrap(),
        Some(day("2025-03-31"))
    );
    assert_eq!(
        s.due_date(day("2025-01-31")).unwrap(),
        Some(day("2025-01-31"))
    );
}
#[test]
fn leap_year_renewal_returns_to_feb_29() {
    let mut s = sample();
    s.expires_at = Some("2024-02-29".into());
    assert_eq!(
        s.due_date(day("2025-02-01")).unwrap(),
        Some(day("2025-02-28"))
    );
    assert_eq!(
        s.due_date(day("2028-02-01")).unwrap(),
        Some(day("2028-02-29"))
    );
}
#[test]
fn nonrenewing_expires_after_expiry_day() {
    let mut s = sample();
    s.auto_renew = false;
    s.expires_at = Some("2026-10-16".into());
    assert!(s.is_current(day("2026-10-16")).unwrap());
    assert!(!s.is_current(day("2026-10-17")).unwrap());
    assert_eq!(s.annual_cost(day("2026-10-17")).unwrap().0, Decimal::ZERO);
    s.expires_at = None;
    assert!(s.validate().is_err());
}
#[test]
fn lifetime_cost_decreases_and_handles_first_day() {
    let mut s = sample();
    s.kind = "lifetime".into();
    s.auto_renew = false;
    s.purchased_at = "2026-01-01".into();
    s.amount = "365".into();
    s.reimbursement = "percent".into();
    s.reimbursement_value = "50".into();
    s.validate().unwrap();
    assert_eq!(
        s.annual_cost(day("2026-01-01")).unwrap().0,
        Decimal::from(133225)
    );
    assert_eq!(
        s.annual_cost(day("2026-12-31")).unwrap().0,
        Decimal::from(365)
    );
    assert_eq!(
        s.annual_cost(day("2026-12-31")).unwrap().1,
        Decimal::from_str("182.5").unwrap()
    );
    assert_eq!(s.annual_cost(day("2025-12-31")).unwrap().0, Decimal::ZERO);
}
#[test]
fn reimbursement_limits_and_decimal_precision() {
    let mut s = sample();
    s.reimbursement_value = "1201".into();
    assert!(s.validate().is_err());
    s.reimbursement = "percent".into();
    s.reimbursement_value = "101".into();
    assert!(s.validate().is_err());
    s.reimbursement = "none".into();
    s.amount = "0.00000001".into();
    s.currency = "BTC".into();
    assert!(s.validate().is_ok());
    s.amount = "NaN".into();
    assert!(s.validate().is_err());
}
#[test]
fn missing_rates_never_silently_zero_totals() {
    let mut db = Database::default();
    let mut s = sample();
    s.currency = "USD".into();
    db.subscriptions.push(s);
    let r = db.summary("CNY", "month", day("2026-10-16")).unwrap();
    assert!(r["bill"].is_null());
    assert_eq!(r["missingRates"][0], "one");
    db.settings.fixed_rates.insert("CNY".into(), "0.125".into());
    let r = db.summary("CNY", "month", day("2026-10-16")).unwrap();
    assert_eq!(r["bill"], "800");
}
#[test]
fn fixed_crypto_rate_and_full_reimbursement() {
    let mut db = Database::default();
    let mut s = sample();
    s.amount = "0.01".into();
    s.currency = "BTC".into();
    s.reimbursement = "full".into();
    s.fixed_usd_rate = Some("60000".into());
    db.subscriptions.push(s);
    let r = db.summary("USD", "year", day("2026-10-16")).unwrap();
    assert_eq!(
        Decimal::from_str(r["bill"].as_str().unwrap()).unwrap(),
        Decimal::from(600)
    );
    assert_eq!(
        Decimal::from_str(r["actual"].as_str().unwrap()).unwrap(),
        Decimal::ZERO
    );
}
#[test]
fn writes_are_backed_up_and_stale_edit_fails() {
    let (dir, store) = store();
    store.save(sample(), 0).unwrap();
    assert!(dir.path().join("subscriptions.json.bak").exists());
    assert!(store.save(sample(), 0).is_err());
    assert_eq!(store.read().unwrap().0.subscriptions.len(), 1);
}
#[test]
fn malformed_import_does_not_replace_saved_data() {
    let (dir, store) = store();
    store.save(sample(), 0).unwrap();
    let bad = dir.path().join("bad.json");
    std::fs::write(&bad, "{broken").unwrap();
    assert!(store.preview_file(bad).is_err());
    assert_eq!(store.read().unwrap().0.subscriptions.len(), 1);
}
#[test]
fn mcp_retries_are_idempotent_and_collisions_fail() {
    let (_dir, store) = store();
    store
        .update(|db| {
            db.settings.mcp_enabled = true;
            Ok(())
        })
        .unwrap();
    let r = store
        .import_batch("batch-1".into(), vec![sample()], false)
        .unwrap();
    assert_eq!(r["added"], 1);
    let r = store
        .import_batch("batch-1".into(), vec![sample()], false)
        .unwrap();
    assert_eq!(r["replayed"], true);
    let mut other = sample();
    other.amount = "1300".into();
    assert!(store
        .import_batch("batch-1".into(), vec![other], false)
        .is_err());
    assert_eq!(store.read().unwrap().0.subscriptions.len(), 1);
}
#[test]
fn possible_duplicates_need_explicit_opt_in() {
    let (_dir, store) = store();
    store
        .update(|db| {
            db.settings.mcp_enabled = true;
            Ok(())
        })
        .unwrap();
    store
        .import_batch("batch-1".into(), vec![sample()], false)
        .unwrap();
    let mut other = sample();
    other.id = "two".into();
    assert!(store
        .import_batch("batch-2".into(), vec![other.clone()], false)
        .is_err());
    assert!(store
        .import_batch("batch-2".into(), vec![other], true)
        .is_ok());
}
#[test]
fn concurrent_writers_retain_all_records() {
    let (dir, store) = store();
    let root = dir.path().to_path_buf();
    let handles: Vec<_> = (0..8)
        .map(|n| {
            let root = root.clone();
            std::thread::spawn(move || {
                Store::at(root)
                    .update(|db| {
                        let mut s = sample();
                        s.id = n.to_string();
                        db.subscriptions.push(s);
                        Ok(())
                    })
                    .unwrap()
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let db = store.read().unwrap().0;
    assert_eq!(db.subscriptions.len(), 8);
    assert_eq!(db.revision, 8);
}
#[test]
fn move_retains_original_and_missing_location_errors() {
    let (dir, store) = store();
    store.save(sample(), 0).unwrap();
    let dest = dir.path().join("moved");
    store.move_to(dest.clone()).unwrap();
    assert!(dir.path().join("subscriptions.json").exists());
    assert_eq!(store.read().unwrap().0.subscriptions.len(), 1);
    std::fs::remove_file(dest.join("subscriptions.json")).unwrap();
    assert!(store.read().is_err());
}
