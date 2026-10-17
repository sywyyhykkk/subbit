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
fn spending_levels_accept_converted_totals_and_respect_basis_and_thresholds() {
    let mut db = Database::default();
    let mut s = sample();
    s.amount = "20".into(); s.currency = "USD".into(); s.cycle_unit = "month".into();
    s.reimbursement = "none".into(); s.reimbursement_value = "0".into();
    db.settings.fixed_rates.insert("CNY".into(), "0.14".into());
    db.subscriptions.push(s);
    assert_eq!(db.spending_level(day("2026-10-17")).unwrap().as_deref(), Some("数字玩家"));
    db.subscriptions[0].amount = "14".into();
    assert_eq!(db.spending_level(day("2026-10-17")).unwrap().as_deref(), Some("数字玩家"));
    db.subscriptions[0].reimbursement = "full".into();
    assert_eq!(db.spending_level(day("2026-10-17")).unwrap().as_deref(), Some("订阅萌新"));
    db.settings.level_basis = "bill".into();
    assert_eq!(db.spending_level(day("2026-10-17")).unwrap().as_deref(), Some("数字玩家"));
    db.settings.show_level = false;
    assert_eq!(db.spending_level(day("2026-10-17")).unwrap(), None);
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

#[test]
fn ui_snapshot_omits_import_receipts_without_changing_backups_or_replays() {
    let (_dir, store) = store();
    store.update(|db| { db.settings.mcp_enabled = true; Ok(()) }).unwrap();
    store.import_batch("batch-view".into(), vec![sample()], false).unwrap();
    let (_, path) = store.read().unwrap();
    let before = std::fs::read(&path).unwrap();
    let backup_before = std::fs::read(path.with_extension("json.bak")).unwrap();
    let snapshot = subbit_lib::view::load(&store, None, None, None, day("2026-10-18"))
        .unwrap().unwrap();
    let value = serde_json::to_value(snapshot).unwrap();
    assert!(value["database"].get("imports").is_none());
    assert_eq!(value["database"]["subscriptions"].as_array().unwrap().len(), 1);
    assert_eq!(value["summary"]["actual"], "50");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(std::fs::read(path.with_extension("json.bak")).unwrap(), backup_before);
    assert!(store.read().unwrap().0.imports.contains_key("batch-view"));
    assert_eq!(store.import_batch("batch-view".into(), vec![sample()], false).unwrap()["replayed"], true);
}

fn snapshot_key(store: &Store) -> subbit_lib::view::SnapshotKey {
    let snapshot = subbit_lib::view::load(store, Some("CNY"), Some("month"), None, day("2026-10-18"))
        .unwrap().unwrap();
    serde_json::from_value(serde_json::to_value(snapshot).unwrap()["key"].clone()).unwrap()
}

#[test]
fn unchanged_snapshot_returns_nothing_but_view_changes_and_new_days_refresh() {
    let (_dir, store) = store();
    store.save(sample(), 0).unwrap();
    let key = snapshot_key(&store);
    let load = |currency, period, day| subbit_lib::view::load(&store, currency, period, Some(&key), day);
    assert!(load(Some("CNY"), Some("month"), day("2026-10-18")).unwrap().is_none());
    assert!(load(Some("USD"), Some("month"), day("2026-10-18")).unwrap().is_some());
    assert!(load(Some("CNY"), Some("year"), day("2026-10-18")).unwrap().is_some());
    assert!(load(Some("CNY"), Some("month"), day("2026-10-19")).unwrap().is_some());
    // Reopening settings must still read the saved defaults, not the current filters.
    assert!(load(None, None, day("2026-10-18")).unwrap().is_some());
}

#[test]
fn snapshot_detects_another_writer_and_same_length_external_edits() {
    let (dir, store) = store();
    store.save(sample(), 0).unwrap();
    let key = snapshot_key(&store);
    let other_writer = Store::at(dir.path().into());
    let mut changed = sample();
    changed.amount = "2400".into();
    other_writer.save(changed, 1).unwrap();
    let snapshot = subbit_lib::view::load(&store, Some("CNY"), Some("month"), Some(&key), key.day)
        .unwrap().unwrap();
    assert_eq!(serde_json::to_value(snapshot).unwrap()["summary"]["bill"], "200");

    let key = snapshot_key(&store);
    let (mut db, path) = store.read().unwrap();
    db.subscriptions[0].amount = "3600".into();
    write_json(&path, &db, false).unwrap();
    // Keep revision and byte length unchanged to exercise the modification-time check.
    assert_eq!(std::fs::metadata(&path).unwrap().len(), key.file.length.unwrap());
    std::fs::File::options().write(true).open(&path).unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(2_000_000_000)).unwrap();
    let snapshot = subbit_lib::view::load(&store, Some("CNY"), Some("month"), Some(&key), key.day)
        .unwrap().unwrap();
    assert_eq!(serde_json::to_value(snapshot).unwrap()["summary"]["bill"], "300");
}

#[test]
fn snapshot_detects_location_changes_and_does_not_hide_missing_files() {
    let (dir, store) = store();
    store.save(sample(), 0).unwrap();
    let key = snapshot_key(&store);
    let destination = dir.path().join("moved");
    store.move_to(destination.clone()).unwrap();
    let snapshot = subbit_lib::view::load(&store, Some("CNY"), Some("month"), Some(&key), key.day)
        .unwrap().unwrap();
    let value = serde_json::to_value(snapshot).unwrap();
    assert_eq!(value["path"], destination.join("subscriptions.json").to_str().unwrap());
    let key = snapshot_key(&store);
    std::fs::remove_file(destination.join("subscriptions.json")).unwrap();
    assert!(subbit_lib::view::load(&store, Some("CNY"), Some("month"), Some(&key), key.day).is_err());
}

#[test]
fn domain_records_preserve_legacy_data_and_validate_registrars() {
    let original = sample();
    assert!(original.registrar.is_none());
    assert!(serde_json::to_value(&original).unwrap().get("registrar").is_none());
    let mut domain = original.clone();
    domain.category = "domain".into();
    domain.name = "example.com".into();
    domain.registrar = Some("Example Registrar".into());
    domain.validate().unwrap();
    let (dir, store) = store();
    store.save(domain.clone(), 0).unwrap();
    assert_eq!(store.read().unwrap().0.subscriptions[0], domain);
    assert!(dir.path().join("subscriptions.json.bak").exists());
    for name in ["https://example.com", "example.com/path", "127.0.0.1", "-bad.com", "a..com"] {
        domain.name = name.into();
        assert!(domain.validate().is_err(), "{name}");
    }
    domain.name = "例子.中国".into();
    domain.validate().unwrap();
    domain.kind = "lifetime".into();
    assert!(domain.validate().is_err());
}
