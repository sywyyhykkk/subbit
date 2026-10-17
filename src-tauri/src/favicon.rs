use crate::{model::{validate_domain, Result}, network};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};

pub fn fetch(domain: &str, icon_url: Option<&str>) -> Result<Value> {
    validate_domain(domain)?;
    network::require_online()?;
    let base = reqwest::Url::parse(&format!("https://{domain}/")).map_err(|e| e.to_string())?;
    let url = match icon_url {
        Some(value) => base.join(value).map_err(|e| e.to_string())?,
        None => base,
    };
    if url.scheme() != "https" || url.username() != "" || url.password().is_some() {
        return Err("Website icons require HTTPS".into());
    }
    let limit = if icon_url.is_some() { 150_000 } else { 262_144 };
    let response = network::download(url.as_str(), limit)?;
    let final_url = response.url;
    let mime = response.mime;
    let bytes = response.bytes;
    if icon_url.is_none() {
        if !mime.contains("html") { return Err("Website did not return HTML".into()); }
        return Ok(json!({"url":final_url,"html":String::from_utf8_lossy(&bytes)}));
    }
    // Only raster images cross into the WebView; normalize ICO MIME types for decoding.
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { "image/png" }
        else if bytes.starts_with(&[0xff, 0xd8, 0xff]) { "image/jpeg" }
        else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") { "image/webp" }
        else if bytes.starts_with(&[0, 0, 1, 0]) { "image/x-icon" }
        else { return Err("Unsupported website icon".into()); };
    Ok(json!({"data":format!("data:{mime};base64,{}", STANDARD.encode(bytes))}))
}
