fn main() {
    use base64::Engine;
    println!("cargo:rerun-if-changed=../catalog");
    let categories: serde_json::Value =
        serde_json::from_slice(&std::fs::read("../catalog/categories.json").unwrap()).unwrap();
    let mut paths: Vec<_> = std::fs::read_dir("../catalog/services")
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let services: Vec<serde_json::Value> = paths
        .iter()
        .map(|p| serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap())
        .collect();
    let icons: std::collections::BTreeMap<String, String> = services.iter().map(|service| {
        let path = service["icon"].as_str().expect("service icon");
        let mime = if path.ends_with(".svg") { "image/svg+xml" } else { "image/png" };
        let bytes = std::fs::read(format!("..{path}")).expect("bundled icon");
        (path.into(), format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
    }).collect();
    std::fs::write(
        std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("catalog.json"),
        serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"categories":categories,"services":services,"icons":icons}))
            .unwrap(),
    )
    .unwrap();
    tauri_build::build()
}
