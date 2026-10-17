use crate::{
    model::{decimal, Result},
    store::{config_root, write_json},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, OpenOptions},
    io::Read,
    path::PathBuf,
    time::Duration,
};

const URL: &str = "https://raw.githubusercontent.com/sywyyhykkk/subbit/catalog/catalog.json";
const MAX_BYTES: u64 = 8_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub schema_version: u32,
    pub categories: Vec<Value>,
    pub services: Vec<Value>,
    pub icons: BTreeMap<String, String>,
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value[key]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 2000)
        .ok_or_else(|| format!("Invalid catalog field: {key}"))
}
fn id(value: &Value) -> Result<&str> {
    let id = text(value, "id")?;
    if id.len() > 100
        || !id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err("Invalid catalog ID".into());
    }
    Ok(id)
}
impl Catalog {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || self.categories.is_empty()
            || self.categories.len() > 200
            || self.services.is_empty()
            || self.services.len() > 5000
            || self.icons.len() > 5000
        {
            return Err("Unsupported or incomplete catalog".into());
        }
        let mut categories = HashSet::new();
        for category in &self.categories {
            if !categories.insert(id(category)?) {
                return Err("Duplicate category".into());
            }
            for field in ["zh", "en", "icon"] {
                text(category, field)?;
            }
        }
        if !categories.contains("other") {
            return Err("Missing custom-service category".into());
        }
        let mut services = HashSet::new();
        for service in &self.services {
            if !services.insert(id(service)?) {
                return Err("Duplicate service".into());
            }
            text(service, "name")?;
            if !categories.contains(text(service, "category")?)
                || !text(service, "source")?.starts_with("https://")
            {
                return Err("Invalid service category or source".into());
            }
            let aliases = service["aliases"].as_array().ok_or("Invalid aliases")?;
            if aliases.len() > 100
                || aliases
                    .iter()
                    .any(|v| v.as_str().is_none_or(|s| s.len() > 200))
            {
                return Err("Invalid aliases".into());
            }
            let icon = text(service, "icon")?;
            if !icon.starts_with("/catalog/icons/")
                || icon.contains("..")
                || !self.icons.contains_key(icon)
            {
                return Err("Missing service icon".into());
            }
            let plans = service["plans"]
                .as_array()
                .filter(|p| !p.is_empty() && p.len() <= 200)
                .ok_or("Invalid plans")?;
            let mut ids = HashSet::new();
            for plan in plans {
                if !ids.insert(id(plan)?) {
                    return Err("Duplicate plan".into());
                }
                text(plan, "name")?;
                let kind = text(plan, "kind")?;
                if !["recurring", "lifetime"].contains(&kind) || !plan["retired"].is_boolean() {
                    return Err("Invalid plan type".into());
                }
                if let Some(cycle) = plan.get("cycleUnit") {
                    if !["day", "week", "month", "year"].contains(&cycle.as_str().unwrap_or("")) {
                        return Err("Invalid plan cycle".into());
                    }
                }
                if let Some(prices) = plan.get("prices") {
                    let prices = prices
                        .as_array()
                        .filter(|p| !p.is_empty() && p.len() <= 100)
                        .ok_or("Invalid prices")?;
                    let mut cycles = HashSet::new();
                    for price in prices {
                        if decimal(text(price, "amountUsd")?)? <= rust_decimal::Decimal::ZERO {
                            return Err("Invalid price".into());
                        }
                        let unit = price["cycleUnit"].as_str().unwrap_or("");
                        let count = price
                            .get("cycleCount")
                            .map_or(Some(1), Value::as_u64)
                            .ok_or("Invalid cycle count")?;
                        if kind == "lifetime" {
                            if price.get("cycleUnit").is_some() || price.get("cycleCount").is_some()
                            {
                                return Err("Invalid lifetime price".into());
                            }
                        } else if !["day", "week", "month", "year"].contains(&unit)
                            || !(1..=120).contains(&count)
                        {
                            return Err("Invalid price cycle".into());
                        }
                        if !cycles.insert((unit, count)) {
                            return Err("Duplicate price cycle".into());
                        }
                    }
                }
            }
        }
        for (path, data) in &self.icons {
            let prefix = if path.ends_with(".svg") {
                "data:image/svg+xml;base64,"
            } else if path.ends_with(".png") {
                "data:image/png;base64,"
            } else {
                return Err("Unsupported catalog icon".into());
            };
            let bytes = STANDARD
                .decode(data.strip_prefix(prefix).ok_or("Invalid icon data URL")?)
                .map_err(|_| "Invalid icon encoding")?;
            if bytes.is_empty() || bytes.len() > 150_000 {
                return Err("Invalid icon size".into());
            }
            if path.ends_with(".png") {
                if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
                    return Err("Invalid PNG icon".into());
                }
            } else if !std::str::from_utf8(&bytes)
                .is_ok_and(|svg| svg.contains("<svg") && svg.contains("</svg>"))
            {
                return Err("Invalid SVG icon".into());
            }
        }
        Ok(())
    }
}

pub fn bundled() -> Catalog {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/catalog.json")))
        .expect("embedded catalog")
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogState {
    pub catalog: Catalog,
    pub updated_at: Option<i64>,
}

pub struct CatalogCache {
    root: PathBuf,
}
impl CatalogCache {
    pub fn system() -> Result<Self> {
        Ok(Self::at(config_root()?.join("catalog-cache")))
    }
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn load(&self) -> CatalogState {
        let cached = (|| -> Result<CatalogState> {
            let file = fs::File::open(self.root.join("catalog.json")).map_err(|e| e.to_string())?;
            let mut bytes = Vec::new();
            file.take(25_000_001)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 25_000_000 {
                return Err("Catalog too large".into());
            }
            let state: CatalogState = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            state.catalog.validate()?;
            Ok(state)
        })();
        let mut state = cached.unwrap_or_else(|_| CatalogState {
            catalog: bundled(),
            updated_at: None,
        });
        if !state.catalog.categories.iter().any(|c| c["id"] == "domain") {
            state.catalog.categories.push(serde_json::json!({"id":"domain","zh":"域名","en":"Domains","icon":"◎"}));
        }
        state
    }
    pub fn refresh(&self, force: bool) -> Result<CatalogState> {
        self.refresh_from(force, URL)
    }
    fn refresh_from(&self, force: bool, url: &str) -> Result<CatalogState> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("catalog.lock"))
            .map_err(|e| e.to_string())?;
        lock.lock_exclusive().map_err(|e| e.to_string())?;
        let current = self.load();
        let now = chrono::Utc::now().timestamp();
        let check_path = self.root.join("checked.json");
        let last: Option<i64> = fs::read(&check_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        if !force && last.is_some_and(|t| now >= t && now - t < 900) {
            return Ok(current);
        }
        // A failed request is retried after 15 minutes; offline window opens stay instant.
        write_json(&check_path, &now, false)?;
        crate::init_tls();
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("Subbit/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| e.to_string())?;
        let response = client
            .get(url)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| e.to_string())?;
        let mut bytes = Vec::new();
        response
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err("Catalog exceeds 8 MB".into());
        }
        let catalog: Catalog = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        catalog.validate()?;
        let state = CatalogState {
            updated_at: if catalog == current.catalog && current.updated_at.is_some() {
                current.updated_at
            } else {
                Some(now)
            },
            catalog,
        };
        // One atomic file contains metadata AND icons. The asset store is never opened here.
        write_json(&self.root.join("catalog.json"), &state, false)?;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, net::TcpListener};

    fn server(status: &str, body: Vec<u8>) -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/catalog.json", listener.local_addr().unwrap());
        let status = status.to_string();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request);
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(&body);
        });
        (url, handle)
    }
    fn download(cache: &CatalogCache, catalog: &Catalog) -> Result<CatalogState> {
        let (url, handle) = server("200 OK", serde_json::to_vec(catalog).unwrap());
        let result = cache.refresh_from(true, &url);
        handle.join().unwrap();
        result
    }
    fn updated() -> Catalog {
        let mut catalog = bundled();
        catalog.categories.push(
            serde_json::json!({"id":"new-category","zh":"新分类","en":"New category","icon":"+"}),
        );
        let mut service = catalog.services[0].clone();
        service["id"] = "new-service".into();
        service["category"] = "new-category".into();
        service["name"] = "New service".into();
        service["icon"] = "/catalog/icons/new-service.svg".into();
        catalog.icons.insert("/catalog/icons/new-service.svg".into(), format!("data:image/svg+xml;base64,{}", STANDARD.encode(b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"24\" height=\"24\"><circle cx=\"12\" cy=\"12\" r=\"8\"/></svg>")));
        catalog.services.push(service);
        catalog
    }

    #[test]
    fn embedded_catalog_is_complete_and_offline_fallback_does_not_write() {
        bundled().validate().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let cache = CatalogCache::at(dir.path().join("catalog-cache"));
        assert_eq!(cache.load().catalog, bundled());
        assert!(cache.load().updated_at.is_none());
        assert!(!dir.path().join("catalog-cache").exists());
    }

    #[test]
    fn new_categories_services_and_icons_survive_restart_without_touching_assets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("subscriptions.json");
        write_json(&path, &crate::model::Database::default(), false).unwrap();
        write_json(&dir.path().join("location.json"), &path, false).unwrap();
        let store = crate::store::Store::at(dir.path().into());
        store
            .update(|db| {
                db.settings.language = "en".into();
                Ok(())
            })
            .unwrap();
        store
            .update(|db| {
                db.settings.theme = "dark".into();
                Ok(())
            })
            .unwrap();
        let (_, path) = store.read().unwrap();
        let files = [
            path.clone(),
            path.with_extension("json.bak"),
            dir.path().join("location.json"),
        ];
        let before: Vec<_> = files.iter().map(|p| fs::read(p).unwrap()).collect();
        let root = dir.path().join("catalog-cache");
        let expected = updated();
        download(&CatalogCache::at(root.clone()), &expected).unwrap();
        let cached = CatalogCache::at(root).load();
        assert_eq!(cached.catalog, expected);
        assert!(cached.updated_at.is_some());
        for (file, bytes) in files.iter().zip(before) {
            assert_eq!(fs::read(file).unwrap(), bytes);
        }
        assert_eq!(store.read().unwrap().0.revision, 2);
    }

    #[test]
    fn failed_or_incomplete_updates_keep_the_entire_last_good_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CatalogCache::at(dir.path().into());
        let expected = updated();
        download(&cache, &expected).unwrap();
        let before = fs::read(dir.path().join("catalog.json")).unwrap();
        let mut invalid = expected.clone();
        invalid.icons.clear();
        assert!(download(&cache, &invalid).is_err());
        invalid = expected.clone();
        invalid.schema_version = 2;
        assert!(download(&cache, &invalid).is_err());
        invalid = expected.clone();
        invalid.services[0]["plans"][0]["prices"] =
            serde_json::json!([{"amountUsd":"20","cycleUnit":"fortnight"}]);
        assert!(download(&cache, &invalid).is_err());
        for (status, body) in [
            ("500 Internal Server Error", b"failure".to_vec()),
            ("200 OK", b"{\"schemaVersion\":1,".to_vec()),
            ("200 OK", vec![b' '; MAX_BYTES as usize + 1]),
        ] {
            let (url, handle) = server(status, body);
            assert!(cache.refresh_from(true, &url).is_err());
            handle.join().unwrap();
        }
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let unavailable = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        assert!(cache.refresh_from(true, &unavailable).is_err());
        assert_eq!(cache.load().catalog, expected);
        assert_eq!(fs::read(dir.path().join("catalog.json")).unwrap(), before);
        // Automatic opens are throttled after failure; manual refresh can retry immediately.
        assert_eq!(
            cache.refresh_from(false, &unavailable).unwrap().catalog,
            expected
        );
        download(&cache, &bundled()).unwrap();
        assert_eq!(cache.load().catalog, bundled());
    }

    #[test]
    fn corrupt_cache_falls_back_and_can_be_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let cache = CatalogCache::at(dir.path().into());
        fs::write(dir.path().join("catalog.json"), b"incomplete").unwrap();
        assert_eq!(cache.load().catalog, bundled());
        download(&cache, &updated()).unwrap();
        assert_eq!(cache.load().catalog, updated());
    }
}
