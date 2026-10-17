use chrono::{Datelike, Duration, Local, Months, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};

pub type Result<T> = std::result::Result<T, String>;
pub const CURRENCIES: &[&str] = &[
    "CNY", "USD", "EUR", "GBP", "JPY", "HKD", "TWD", "KRW", "SGD", "PHP", "MYR", "THB", "IDR",
    "VND", "TRY", "INR", "BRL", "ARS", "MXN", "AUD", "CAD", "NZD", "CHF", "AED", "SAR", "ZAR",
    "USDT", "USDC", "BTC", "ETH", "SOL",
];
pub fn today() -> NaiveDate {
    Local::now().date_naive()
}
pub fn decimal(s: &str) -> Result<Decimal> {
    let n = Decimal::from_str(s).map_err(|_| format!("Invalid decimal: {s}"))?;
    if n < Decimal::ZERO || n > Decimal::from(1_000_000_000_000u64) || n.scale() > 12 {
        return Err("Amount must be 0–1,000,000,000,000, with at most 12 decimals".into());
    }
    Ok(n)
}
pub fn date(s: &str) -> Result<NaiveDate> {
    let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| "Invalid date; use YYYY-MM-DD".to_string())?;
    if !(1900..=9999).contains(&d.year()) {
        return Err("Date must be between 1900 and 9999".into());
    }
    Ok(d)
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub service_id: Option<String>,
    pub category: String,
    pub plan: String,
    pub icon: Option<String>,
    pub amount: String,
    pub currency: String,
    pub kind: String,
    pub cycle_unit: String,
    pub cycle_count: u32,
    pub purchased_at: String,
    pub expires_at: Option<String>,
    pub auto_renew: bool,
    pub active: bool,
    pub reimbursement: String,
    pub reimbursement_value: String,
    pub payment_method: String,
    pub notes: String,
    pub fixed_usd_rate: Option<String>,
}
impl Subscription {
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || self.id.len() > 100
            || self.name.trim().is_empty()
            || self.name.len() > 200
        {
            return Err("ID and service name are required".into());
        }
        if !CURRENCIES.contains(&self.currency.as_str()) {
            return Err("Unsupported currency".into());
        }
        if self.category.len() > 100
            || self.plan.len() > 200
            || self.notes.len() > 4000
            || self.payment_method.len() > 100
        {
            return Err("Text is too long".into());
        }
        let amount = decimal(&self.amount)?;
        let reimbursement = decimal(&self.reimbursement_value)?;
        match self.reimbursement.as_str() {
            "none" | "full" => {}
            "fixed" if reimbursement <= amount => {}
            "percent" if reimbursement <= Decimal::from(100) => {}
            _ => return Err("Invalid reimbursement amount or type".into()),
        }
        if !["recurring", "lifetime"].contains(&self.kind.as_str()) {
            return Err("Invalid purchase type".into());
        }
        if !["day", "week", "month", "year"].contains(&self.cycle_unit.as_str())
            || !(1..=120).contains(&self.cycle_count)
        {
            return Err("Invalid billing cycle (1–120)".into());
        }
        let purchased = date(&self.purchased_at)?;
        if let Some(expiry) = &self.expires_at {
            if date(expiry)? < purchased {
                return Err("Expiry precedes purchase date".into());
            }
        }
        if self.kind == "lifetime" && (self.expires_at.is_some() || self.auto_renew) {
            return Err("Lifetime purchases cannot expire or renew".into());
        }
        if self.kind == "recurring" && !self.auto_renew && self.expires_at.is_none() {
            return Err("An expiry date is required when auto-renew is off".into());
        }
        if let Some(rate) = &self.fixed_usd_rate {
            if decimal(rate)? == Decimal::ZERO {
                return Err("Fixed rate must be positive".into());
            }
        }
        if let Some(icon) = &self.icon {
            if icon.len() > 150_000
                || ![
                    "data:image/png;base64,",
                    "data:image/jpeg;base64,",
                    "data:image/webp;base64,",
                ]
                .iter()
                .any(|p| icon.starts_with(p))
            {
                return Err("Icon must be a small PNG, JPG or WebP data URL".into());
            }
        }
        Ok(())
    }
    pub fn due_date(&self, now: NaiveDate) -> Result<Option<NaiveDate>> {
        if self.kind == "lifetime" {
            return Ok(None);
        }
        let anchor = self
            .expires_at
            .as_deref()
            .map(date)
            .transpose()?
            .unwrap_or(date(&self.purchased_at)?);
        let first = if self.expires_at.is_some() { 0 } else { 1 };
        if !self.auto_renew {
            return Ok(Some(anchor));
        }
        // Always offset from the original anchor: Jan 31 -> Feb 28 -> Mar 31.
        for n in first..=100_000u32 {
            let count = self.cycle_count.checked_mul(n).ok_or("Cycle overflow")?;
            let next = match self.cycle_unit.as_str() {
                "month" => anchor.checked_add_months(Months::new(count)),
                "year" => anchor.checked_add_months(Months::new(
                    count.checked_mul(12).ok_or("Cycle overflow")?,
                )),
                "week" => anchor.checked_add_signed(Duration::days(i64::from(count) * 7)),
                _ => anchor.checked_add_signed(Duration::days(i64::from(count))),
            }
            .ok_or("Date out of range")?;
            if next >= now {
                return Ok(Some(next));
            }
        }
        Err("Renewal date is out of range".into())
    }
    pub fn is_current(&self, now: NaiveDate) -> Result<bool> {
        Ok(self.active
            && date(&self.purchased_at)? <= now
            && (self.kind == "lifetime"
                || self.auto_renew
                || self
                    .expires_at
                    .as_deref()
                    .map(date)
                    .transpose()?
                    .is_some_and(|d| d >= now)))
    }
    pub fn annual_cost(&self, now: NaiveDate) -> Result<(Decimal, Decimal)> {
        if !self.is_current(now)? {
            return Ok((Decimal::ZERO, Decimal::ZERO));
        }
        let bill = decimal(&self.amount)?;
        let v = decimal(&self.reimbursement_value)?;
        let actual = match self.reimbursement.as_str() {
            "full" => Decimal::ZERO,
            "fixed" => bill - v,
            "percent" => bill * (Decimal::ONE - v / Decimal::from(100)),
            _ => bill,
        };
        let multiplier = if self.kind == "lifetime" {
            Decimal::from(365) / Decimal::from((now - date(&self.purchased_at)?).num_days() + 1)
        } else {
            let count = Decimal::from(self.cycle_count);
            match self.cycle_unit.as_str() {
                "year" => Decimal::ONE / count,
                "month" => Decimal::from(12) / count,
                "week" => Decimal::from(365) / (count * Decimal::from(7)),
                _ => Decimal::from(365) / count,
            }
        };
        Ok((bill * multiplier, actual * multiplier))
    }
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct Settings {
    pub currency: String,
    pub period: String,
    pub language: String,
    pub theme: String,
    pub currency_symbol: bool,
    pub show_level: bool,
    pub level_basis: String,
    pub mcp_enabled: bool,
    pub auto_rates: bool,
    pub fixed_rates: BTreeMap<String, String>,
    pub levels: Vec<Level>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Level {
    pub above: String,
    pub name: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            currency: "CNY".into(),
            period: "month".into(),
            language: "system".into(),
            theme: "system".into(),
            currency_symbol: true,
            show_level: true,
            level_basis: "actual".into(),
            mcp_enabled: false,
            auto_rates: true,
            fixed_rates: BTreeMap::new(),
            levels: [("0", "订阅萌新"), ("100", "数字玩家"), ("300", "效率达人"), ("600", "云端收藏家"), ("1000", "数字土豪")]
                .into_iter().map(|(above, name)| Level { above: above.into(), name: name.into() }).collect(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if !CURRENCIES[..26].contains(&self.currency.as_str())
            || !["hour", "month", "year"].contains(&self.period.as_str())
            || !["system", "zh", "en"].contains(&self.language.as_str())
            || !["system", "light", "dark"].contains(&self.theme.as_str())
            || !["actual", "bill"].contains(&self.level_basis.as_str())
        {
            return Err("Invalid settings".into());
        }
        for (k, v) in &self.fixed_rates {
            if !CURRENCIES.contains(&k.as_str())
                || decimal(v)? == Decimal::ZERO
                || (k == "USD" && decimal(v)? != Decimal::ONE)
            {
                return Err("Invalid fixed rate (USD must equal 1)".into());
            }
        }
        for l in &self.levels {
            decimal(&l.above)?;
            if l.name.is_empty() || l.name.len() > 100 {
                return Err("Invalid level name".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rate {
    pub usd: String,
    pub as_of: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub records: Vec<Subscription>,
    pub added: usize,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Database {
    pub schema_version: u32,
    pub revision: u64,
    pub subscriptions: Vec<Subscription>,
    pub settings: Settings,
    pub rates: BTreeMap<String, Rate>,
    pub imports: BTreeMap<String, Receipt>,
}
impl Default for Database {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 0,
            subscriptions: vec![],
            settings: Settings::default(),
            rates: BTreeMap::new(),
            imports: BTreeMap::new(),
        }
    }
}
impl Database {
    pub fn spending_level(&self, now: NaiveDate) -> Result<Option<String>> {
        if !self.settings.show_level { return Ok(None); }
        let summary = self.summary("CNY", "month", now)?;
        // Calculated totals can have more than the 12 decimal places allowed for user input.
        let value = summary[&self.settings.level_basis].as_str()
            .and_then(|s| s.parse::<Decimal>().ok());
        Ok(value.and_then(|value| self.settings.levels.iter()
            .filter(|level| decimal(&level.above).is_ok_and(|threshold| value >= threshold))
            .max_by_key(|level| decimal(&level.above).unwrap_or_default())
            .map(|level| level.name.clone())))
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err("Unsupported data version".into());
        }
        self.settings.validate()?;
        let mut ids = std::collections::HashSet::new();
        for s in &self.subscriptions {
            s.validate()?;
            if !ids.insert(&s.id) {
                return Err("Duplicate subscription ID".into());
            }
        }
        for (k, v) in &self.rates {
            if !CURRENCIES.contains(&k.as_str()) || decimal(&v.usd)? == Decimal::ZERO {
                return Err("Invalid exchange rate".into());
            }
        }
        for r in self.imports.values() {
            for s in &r.records {
                s.validate()?;
            }
        }
        Ok(())
    }
    pub fn usd_rate(&self, currency: &str) -> Option<Decimal> {
        if currency == "USD" {
            return Some(Decimal::ONE);
        }
        self.settings
            .fixed_rates
            .get(currency)
            .or_else(|| self.rates.get(currency).map(|r| &r.usd))
            .and_then(|s| decimal(s).ok())
            .filter(|d| *d > Decimal::ZERO)
    }
    pub fn summary(
        &self,
        currency: &str,
        period: &str,
        now: NaiveDate,
    ) -> Result<serde_json::Value> {
        if !CURRENCIES.contains(&currency) {
            return Err("Unsupported currency".into());
        }
        let divisor = match period {
            "hour" => Decimal::from(8760),
            "month" => Decimal::from(12),
            "year" => Decimal::ONE,
            _ => return Err("Invalid period".into()),
        };
        let mut bill = Decimal::ZERO;
        let mut actual = Decimal::ZERO;
        let mut missing = Vec::new();
        let mut rows = Vec::new();
        let mut active = 0;
        for s in &self.subscriptions {
            let current = s.is_current(now)?;
            if current {
                active += 1;
            }
            let (b, a) = s.annual_cost(now)?;
            let rate = if !current || (s.currency == currency && s.fixed_usd_rate.is_none()) {
                Some(Decimal::ONE)
            } else {
                s.fixed_usd_rate
                    .as_deref()
                    .and_then(|v| decimal(v).ok())
                    .or_else(|| self.usd_rate(&s.currency))
                    .zip(self.usd_rate(currency))
                    .map(|(from, to)| from / to)
            };
            let costs = rate
                .map(|r| -> Result<(Decimal, Decimal)> {
                    Ok((
                        b.checked_mul(r)
                            .and_then(|v| v.checked_div(divisor))
                            .ok_or("Converted amount is too large")?,
                        a.checked_mul(r)
                            .and_then(|v| v.checked_div(divisor))
                            .ok_or("Converted amount is too large")?,
                    ))
                })
                .transpose()?;
            if let Some((b, a)) = costs {
                bill = bill.checked_add(b).ok_or("Total is too large")?;
                actual = actual.checked_add(a).ok_or("Total is too large")?;
            } else {
                missing.push(s.id.clone());
            }
            rows.push(serde_json::json!({"id":s.id,"current":current,"dueDate":s.due_date(now)?.map(|d|d.to_string()),"bill":costs.map(|c|c.0.to_string()),"actual":costs.map(|c|c.1.to_string()),"elapsedDays":(now-date(&s.purchased_at)?).num_days()+1}));
        }
        Ok(
            serde_json::json!({"currency":currency,"period":period,"bill":if missing.is_empty(){Some(bill.to_string())}else{None},"actual":if missing.is_empty(){Some(actual.to_string())}else{None},"reimbursed":if missing.is_empty(){Some((bill-actual).to_string())}else{None},"missingRates":missing,"rows":rows,"activeCount":active}),
        )
    }
}
