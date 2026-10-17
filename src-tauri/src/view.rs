use crate::{
    model::*,
    store::{FileStamp, Store},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotKey {
    pub file: FileStamp,
    pub currency: String,
    pub period: String,
    pub day: NaiveDate,
}

// Import receipts stay on disk for MCP idempotency; the UI never needs them.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseView {
    schema_version: u32,
    revision: u64,
    subscriptions: Vec<Subscription>,
    settings: Settings,
    rates: BTreeMap<String, Rate>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    database: DatabaseView,
    path: PathBuf,
    summary: Value,
    level: Option<String>,
    executable: PathBuf,
    key: SnapshotKey,
}

pub fn load(
    store: &Store,
    currency: Option<&str>,
    period: Option<&str>,
    known: Option<&SnapshotKey>,
    day: NaiveDate,
) -> Result<Option<Snapshot>> {
    let unchanged_view = known.filter(|key| {
        key.day == day
            && currency == Some(key.currency.as_str())
            && period == Some(key.period.as_str())
    });
    let Some((db, file)) = store.read_if_changed(unchanged_view.map(|key| &key.file))? else {
        return Ok(None);
    };
    let currency = currency.unwrap_or(&db.settings.currency);
    let period = period.unwrap_or(&db.settings.period);
    let summary = db.summary(currency, period, day)?;
    let level = db.spending_level(day)?;
    let key = SnapshotKey {
        file,
        currency: currency.into(),
        period: period.into(),
        day,
    };
    Ok(Some(Snapshot {
        path: key.file.path.clone(),
        summary,
        level,
        executable: std::env::current_exe().map_err(|e| e.to_string())?,
        key,
        database: DatabaseView {
            schema_version: db.schema_version,
            revision: db.revision,
            subscriptions: db.subscriptions,
            settings: db.settings,
            rates: db.rates,
        },
    }))
}
