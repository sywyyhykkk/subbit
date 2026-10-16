#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if std::env::args().any(|a| a == "--mcp") {
        if let Err(e) = subbit_lib::mcp::run() {
            eprintln!("{e}");
            std::process::exit(1);
        }
    } else {
        subbit_lib::run();
    }
}
