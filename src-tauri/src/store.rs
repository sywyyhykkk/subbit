use crate::model::*;
use fs2::FileExt;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub struct Store {
    root: PathBuf,
}
pub(crate) fn config_root() -> Result<PathBuf> {
    std::env::var_os("SUBBIT_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir().map(|p| p.join("com.sywyyhykkk.subbit")))
        .ok_or_else(|| "Cannot locate config directory".into())
}
impl Store {
    pub fn system() -> Result<Self> {
        let root = config_root()?;
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        Ok(Self { root })
    }
    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }
    fn lock(&self) -> Result<File> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        let f = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.root.join("store.lock"))
            .map_err(|e| e.to_string())?;
        f.lock_exclusive().map_err(|e| e.to_string())?;
        Ok(f)
    }
    fn location(&self) -> Result<PathBuf> {
        let config = self.root.join("location.json");
        if config.exists() {
            let p: PathBuf = serde_json::from_slice(&fs::read(config).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            if !p.is_file() {
                return Err(format!(
                    "Data file unavailable: {}. Restore it or select its new location.",
                    p.display()
                ));
            }
            return Ok(p);
        }
        let directory = if std::env::var_os("SUBBIT_CONFIG_DIR").is_some() {
            self.root.join("data")
        } else {
            dirs::document_dir()
                .ok_or("Cannot locate Documents folder")?
                .join("Subbit")
        };
        Ok(directory.join("subscriptions.json"))
    }
    fn read_at(path: &Path) -> Result<Database> {
        if !path.exists() {
            return Ok(Database::default());
        }
        let bytes = read_file(path)?;
        let db: Database = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Cannot read data; original file was preserved: {e}"))?;
        db.validate()?;
        Ok(db)
    }
    pub fn read(&self) -> Result<(Database, PathBuf)> {
        let _lock = self.lock()?;
        let path = self.location()?;
        let db = Self::read_at(&path)?;
        Ok((db, path))
    }
    pub fn update<T>(&self, f: impl FnOnce(&mut Database) -> Result<T>) -> Result<T> {
        let _lock = self.lock()?;
        let path = self.location()?;
        let mut db = Self::read_at(&path)?;
        let result = f(&mut db)?;
        db.validate()?;
        db.revision += 1;
        write_json(&path, &db, true)?;
        write_json(&self.root.join("location.json"), &path, false)?;
        Ok(result)
    }
    pub fn save(&self, record: Subscription, revision: u64) -> Result<()> {
        record.validate()?;
        self.update(|db| {
            if db.revision != revision {
                return Err(
                    "Data changed in another window or Agent. Reload before saving.".into(),
                );
            }
            if let Some(old) = db.subscriptions.iter_mut().find(|s| s.id == record.id) {
                *old = record;
            } else {
                db.subscriptions.push(record);
            }
            Ok(())
        })
    }
    pub fn move_to(&self, directory: PathBuf) -> Result<()> {
        let _lock = self.lock()?;
        let old = self.location()?;
        let db = Self::read_at(&old)?;
        let dest = directory.join("subscriptions.json");
        if dest == old {
            return Ok(());
        }
        if dest.exists() {
            return Err(
                "Destination already contains subscriptions.json; use Locate existing data instead"
                    .into(),
            );
        }
        write_json(&dest, &db, false)?;
        write_json(&self.root.join("location.json"), &dest, false)
    }
    pub fn locate(&self, path: PathBuf) -> Result<()> {
        let _lock = self.lock()?;
        if !path.is_file() {
            return Err("Choose an existing JSON file".into());
        }
        Self::read_at(&path)?;
        write_json(&self.root.join("location.json"), &path, false)
    }
    pub fn export(&self, path: PathBuf) -> Result<()> {
        let _lock = self.lock()?;
        let source = self.location()?;
        if path == source
            || path == self.root.join("location.json")
            || path == self.root.join("store.lock")
            || path == source.with_extension("json.bak")
        {
            return Err("Choose a separate export file".into());
        }
        write_json(&path, &Self::read_at(&source)?, false)
    }
    pub fn preview_file(&self, path: PathBuf) -> Result<Database> {
        if !path.is_file() {
            return Err("File does not exist".into());
        }
        Self::read_at(&path)
    }
    pub fn replace(&self, mut replacement: Database, revision: u64) -> Result<()> {
        replacement.validate()?;
        self.update(|db| {
            if db.revision != revision {
                return Err("Data changed; preview the import again".into());
            }
            replacement.revision = db.revision;
            replacement.settings.mcp_enabled = db.settings.mcp_enabled;
            *db = replacement;
            Ok(())
        })
    }
    pub fn preview_batch(db: &Database, records: &[Subscription]) -> Result<Value> {
        if records.is_empty() || records.len() > 1000 {
            return Err("Import 1–1000 records per batch".into());
        }
        let mut ids = std::collections::HashSet::new();
        let mut duplicates = vec![];
        for r in records {
            r.validate()?;
            if !ids.insert(&r.id) {
                return Err("Duplicate IDs within batch".into());
            }
            if let Some(existing) = db.subscriptions.iter().find(|s| s.id == r.id) {
                if existing != r {
                    return Err(format!("ID already exists with different data: {}", r.id));
                }
            }
            if db
                .subscriptions
                .iter()
                .any(|s| s.name.to_lowercase() == r.name.to_lowercase() && s.id != r.id)
            {
                duplicates.push(r.id.clone());
            }
        }
        let skipped = records
            .iter()
            .filter(|r| db.subscriptions.iter().any(|s| s.id == r.id))
            .count();
        Ok(json!({"added":records.len()-skipped,"skipped":skipped,"possibleDuplicates":duplicates}))
    }
    pub fn import_batch(
        &self,
        batch: String,
        records: Vec<Subscription>,
        allow_duplicates: bool,
    ) -> Result<Value> {
        if batch.is_empty() || batch.len() > 100 {
            return Err("A stable importId is required".into());
        }
        self.update(|db|{
            if !db.settings.mcp_enabled {return Err("Enable MCP in Subbit settings first".into());}
            if let Some(receipt)=db.imports.get(&batch){if receipt.records!=records{return Err("importId was already used for different records".into());}return Ok(json!({"added":0,"skipped":records.len(),"replayed":true}));}
            let result=Self::preview_batch(db,&records)?;
            if !allow_duplicates && !result["possibleDuplicates"].as_array().unwrap().is_empty(){return Err("Possible duplicates; preview and explicitly set allowDuplicates to keep separate accounts".into());}
            for r in &records {if !db.subscriptions.iter().any(|s|s.id==r.id){db.subscriptions.push(r.clone());}}
            db.imports.insert(batch,Receipt{records,added:result["added"].as_u64().unwrap() as usize});Ok(result)
        })
    }
}
pub fn read_file(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = vec![];
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(25_000_001)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 25_000_000 {
        return Err("JSON exceeds 25 MB".into());
    }
    Ok(bytes)
}
pub fn write_json(path: &Path, value: &impl serde::Serialize, backup: bool) -> Result<()> {
    let parent = path.parent().ok_or("Invalid path")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > 25_000_000 {
        return Err("Data exceeds 25 MB".into());
    }
    if backup && path.exists() {
        let old = read_file(path)?;
        atomic_write(&path.with_extension("json.bak"), &old)?;
    }
    atomic_write(path, &bytes)
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut tmp = tempfile::NamedTempFile::new_in(path.parent().ok_or("Invalid path")?)
        .map_err(|e| e.to_string())?;
    tmp.write_all(bytes).map_err(|e| e.to_string())?;
    tmp.as_file().sync_all().map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
