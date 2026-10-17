pub mod mcp;
pub mod catalog;
pub mod model;
mod rates;
pub mod network;
pub mod store;
pub mod view;
use model::*;
use serde_json::Value;
use std::path::PathBuf;
use store::Store;
use tauri::{
    menu::{Menu, MenuItem, MenuItemKind},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub fn catalog() -> Value {
    serde_json::to_value(catalog::CatalogCache::system().map(|c| c.load().catalog)
        .unwrap_or_else(|_| catalog::bundled())).expect("catalog serialization")
}
pub(crate) fn init_tls() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
#[tauri::command]
fn offline_mode() -> Result<bool> { network::offline() }
#[tauri::command]
fn set_offline_mode(enabled: bool, app: tauri::AppHandle) -> Result<()> {
    network::set_offline(enabled)?;
    let _ = app.emit("network-changed", enabled);
    Ok(())
}
#[tauri::command]
fn require_online() -> Result<()> { network::require_online() }
#[tauri::command]
fn load_catalog() -> Result<catalog::CatalogState> {
    let mut state = catalog::CatalogCache::system()?.load();
    state.imports.clear();
    Ok(state)
}
#[tauri::command]
async fn refresh_catalog(force: bool) -> Result<catalog::CatalogState> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut state = catalog::CatalogCache::system()?.refresh(force)?;
        state.imports.clear();
        Ok(state)
    })
        .await.map_err(|e| e.to_string())?
}
#[tauri::command]
fn snapshot(currency: Option<String>, period: Option<String>, known: Option<view::SnapshotKey>) -> Result<Option<view::Snapshot>> {
    view::load(&Store::system()?, currency.as_deref(), period.as_deref(), known.as_ref(), today())
}
#[tauri::command]
fn save_subscription(record: Subscription, revision: u64, app: tauri::AppHandle) -> Result<()> {
    Store::system()?.save(record, revision)?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn delete_subscription(id: String, revision: u64, app: tauri::AppHandle) -> Result<()> {
    Store::system()?.update(|db| {
        if db.revision != revision {
            return Err("Data changed; reload before deleting".into());
        }
        db.subscriptions.retain(|s| s.id != id);
        Ok(())
    })?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn save_settings(settings: Settings, revision: u64, app: tauri::AppHandle) -> Result<()> {
    settings.validate()?;
    Store::system()?.update(|db| {
        if db.revision != revision {
            return Err("Data changed; reopen settings before saving".into());
        }
        db.settings = settings;
        Ok(())
    })?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
async fn refresh_rates(force: bool) -> Result<Vec<String>> {
    tauri::async_runtime::spawn_blocking(move || rates::refresh(force))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
fn export_data(path: PathBuf) -> Result<()> {
    Store::system()?.export(path)
}
#[tauri::command]
fn preview_file(path: PathBuf) -> Result<Database> {
    Store::system()?.preview_file(path)
}
#[tauri::command]
fn replace_data(database: Database, revision: u64, app: tauri::AppHandle) -> Result<()> {
    Store::system()?.replace(database, revision)?;
    let _ = app.emit("data-changed", ());
    Ok(())
}
#[tauri::command]
fn move_data(directory: PathBuf) -> Result<()> {
    Store::system()?.move_to(directory)
}
#[tauri::command]
fn locate_data(path: PathBuf) -> Result<()> {
    Store::system()?.locate(path)
}
#[tauri::command]
fn open_main(app: tauri::AppHandle, action: Option<String>) -> Result<()> {
    show_main(&app, action.as_deref().unwrap_or("")).map_err(|e| e.to_string())
}
#[tauri::command]
fn quit(app: tauri::AppHandle) {
    app.exit(0);
}
#[tauri::command]
fn apply_appearance(app: tauri::AppHandle, theme: String, dark: bool) -> Result<()> {
    let theme = match theme.as_str() {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        "system" => None,
        _ => return Err("Invalid appearance".into()),
    };
    app.set_theme(theme);
    #[cfg(target_os = "macos")]
    app.run_on_main_thread(move || {
        use objc2::{MainThreadMarker, AnyThread};
        use objc2_app_kit::{NSApplication, NSImage};
        use objc2_foundation::NSData;
        if let Some(mtm) = MainThreadMarker::new() {
            let bytes: &[u8] = if dark { include_bytes!("../icons/app-dark.png") }
                else { include_bytes!("../icons/app-light.png") };
            if let Some(icon) = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(bytes)) {
                unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&icon)); }
            }
        }
    }).map_err(|e| e.to_string())?;
    #[cfg(not(target_os = "macos"))]
    for window in app.webview_windows().values() {
        let bytes: &[u8] = if dark { include_bytes!("../icons/app-dark.png") }
            else { include_bytes!("../icons/app-light.png") };
        window.set_icon(tauri::image::Image::from_bytes(bytes).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn show_main(app: &tauri::AppHandle, action: &str) -> tauri::Result<()> {
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.destroy();
    }
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
        if ["settings", "add", "about"].contains(&action) {
            window.emit(&format!("open-{action}"), ())?;
        }
        return Ok(());
    }
    WebviewWindowBuilder::new(
        app,
        "main",
        WebviewUrl::App(
            match action {
                "settings" => "index.html?settings=1",
                "add" => "index.html?add=1",
                "about" => "index.html?about=1",
                _ => "index.html",
            }
            .into(),
        ),
    )
    .title("Subbit")
    .inner_size(880., 780.)
    .min_inner_size(620., 520.)
    .center()
    .build()?;
    Ok(())
}
fn show_panel(app: &tauri::AppHandle, position: tauri::PhysicalPosition<f64>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }
    if let Some(window) = app.get_webview_window("panel") {
        window.destroy()?;
        return Ok(());
    }
    let window =
        WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("index.html?panel=1".into()))
            .title("Subbit")
            .inner_size(400., 650.)
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .build()?;
    let monitor = window.available_monitors()?.into_iter().find(|m| {
        let p = m.position();
        let s = m.size();
        position.x >= p.x as f64
            && position.y >= p.y as f64
            && position.x < (p.x as f64 + s.width as f64)
            && position.y < (p.y as f64 + s.height as f64)
    });
    if let Some(m) = monitor {
        let scale = m.scale_factor();
        let p = m.position();
        let s = m.size();
        let width = 400. * scale;
        let height = 650. * scale;
        let x = (position.x - width / 2.)
            .max(p.x as f64 + 8.)
            .min(p.x as f64 + s.width as f64 - width - 8.);
        let y = if position.y > p.y as f64 + s.height as f64 / 2. {
            position.y - height - 16. * scale
        } else {
            position.y + 16. * scale
        };
        window.set_position(tauri::PhysicalPosition::new(
            x as i32,
            y.max(p.y as f64) as i32,
        ))?;
    }
    let panel = window.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Focused(false) = event {
            let _ = panel.destroy();
        }
    });
    window.show()?;
    window.set_focus()?;
    Ok(())
}
pub fn run() {
    init_tls();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = show_main(app, "");
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            offline_mode,
            set_offline_mode,
            require_online,
            load_catalog,
            refresh_catalog,
            snapshot,
            save_subscription,
            delete_subscription,
            save_settings,
            refresh_rates,
            export_data,
            preview_file,
            replace_data,
            move_data,
            locate_data,
            open_main,
            apply_appearance,
            quit
        ])
        .setup(|app| {
            // Route the native About item to the same localized, link-enabled view.
            let app_menu = Menu::default(app.handle())?;
            let about = MenuItem::with_id(app, "about-subbit", "About Subbit", true, None::<&str>)?;
            #[cfg(target_os = "macos")]
            if let Some(MenuItemKind::Submenu(submenu)) = app_menu.items()?.first() {
                submenu.remove_at(0)?;
                submenu.insert(&about, 0)?;
            }
            #[cfg(not(target_os = "macos"))]
            if let Some(MenuItemKind::Submenu(submenu)) = app_menu.items()?.last() {
                submenu.remove_at(0)?;
                submenu.insert(&about, 0)?;
            }
            app.set_menu(app_menu)?;
            app.on_menu_event(|app, event| {
                if event.id.as_ref() == "about-subbit" { let _ = show_main(app, "about"); }
            });
            let open = MenuItem::with_id(app, "open", "打开 Subbit / Open Subbit", true, None::<&str>)?;
            let settings =
                MenuItem::with_id(app, "settings", "设置 / Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 Subbit / Quit Subbit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &settings, &quit])?;
            TrayIconBuilder::with_id("subbit")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray.png"
                ))?)
                .icon_as_template(true)
                .tooltip("Subbit")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        let _ = show_main(app, "");
                    }
                    "settings" => {
                        let _ = show_main(app, "settings");
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        let _ = show_panel(tray.app_handle(), position);
                    }
                })
                .build(app)?;
            if !std::env::args().any(|a| a == "--background") {
                show_main(app.handle(), "")?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("Could not start Subbit")
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
