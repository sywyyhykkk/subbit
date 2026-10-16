fn main() {
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
    std::fs::write(
        std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("catalog.json"),
        serde_json::to_vec(&serde_json::json!({"categories":categories,"services":services}))
            .unwrap(),
    )
    .unwrap();
    tauri_build::build()
}
