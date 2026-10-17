use crate::{model::Result, store::{config_root, write_json}};
use std::{fs, path::Path, time::Duration};

fn read_offline(root: &Path) -> Result<bool> {
    match fs::read(root.join("offline.json")) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}
pub fn offline() -> Result<bool> { read_offline(&config_root()?) }
pub fn set_offline(enabled: bool) -> Result<()> {
    // Application policy is independent of the selected asset file and backup imports.
    write_json(&config_root()?.join("offline.json"), &enabled, true)
}
pub fn require_online() -> Result<()> {
    if offline()? { Err("Offline mode is enabled".into()) } else { Ok(()) }
}
pub fn client() -> Result<reqwest::blocking::Client> {
    require_online()?;
    crate::init_tls();
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("Subbit/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if require_online().is_err() { attempt.stop() }
            else if attempt.previous().len() >= 5 { attempt.error("Too many redirects") }
            else { attempt.follow() }
        }))
        .build().map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn policy_defaults_online_and_corruption_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        assert!(!read_offline(root.path()).unwrap());
        write_json(&root.path().join("offline.json"), &true, true).unwrap();
        assert!(read_offline(root.path()).unwrap());
        fs::write(root.path().join("offline.json"), "broken").unwrap();
        assert!(read_offline(root.path()).is_err());
    }
}
