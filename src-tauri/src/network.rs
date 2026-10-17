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
pub struct Download {
    pub url: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}
pub fn download(url: &str, limit: usize) -> Result<Download> {
    download_with_policy(url, limit, require_online)
}
fn download_with_policy(url: &str, limit: usize, allowed: impl Fn() -> Result<()>) -> Result<Download> {
    allowed()?;
    crate::init_tls();
    // Reuse Tauri's runtime. Dropping this future cancels active network I/O when
    // another window or process enables offline mode, including during redirects/body reads.
    tauri::async_runtime::block_on(async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(concat!("Subbit/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if require_online().is_err() { attempt.stop() }
                else if attempt.previous().len() >= 5 { attempt.error("Too many redirects") }
                else { attempt.follow() }
            })).build().map_err(|e| e.to_string())?;
        let mut request = Box::pin(async {
            allowed()?;
            let mut response = client.get(url).send().await.and_then(|r| r.error_for_status()).map_err(|e| e.to_string())?;
            let url = response.url().to_string();
            let mime = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or("").split(';').next().unwrap_or("").to_ascii_lowercase();
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
                if bytes.len() + chunk.len() > limit { return Err("Network response is too large".into()); }
                bytes.extend_from_slice(&chunk);
            }
            allowed()?;
            Ok(Download { url, mime, bytes })
        });
        let mut interval = tokio::time::interval(Duration::from_millis(25));
        std::future::poll_fn(|cx| {
            while interval.poll_tick(cx).is_ready() {
                if let Err(error) = allowed() { return std::task::Poll::Ready(Err(error)); }
            }
            std::future::Future::poll(request.as_mut(), cx)
        }).await
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switching_offline_cancels_a_pending_response() {
        use std::{io::{Read, Write}, net::TcpListener, sync::{Arc, atomic::{AtomicBool, Ordering}}};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/slow", listener.local_addr().unwrap());
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = cancelled.clone();
        let (finished, wait) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048]; let _ = stream.read(&mut request);
            std::thread::sleep(Duration::from_millis(50));
            flag.store(true, Ordering::SeqCst);
            let _ = wait.recv_timeout(Duration::from_secs(2));
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
        });
        let result = download_with_policy(&url, 1024, || {
            if cancelled.load(Ordering::SeqCst) { Err("Offline mode is enabled".into()) } else { Ok(()) }
        });
        assert_eq!(result.err().as_deref(), Some("Offline mode is enabled"));
        finished.send(()).unwrap();
        server.join().unwrap();
    }
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
