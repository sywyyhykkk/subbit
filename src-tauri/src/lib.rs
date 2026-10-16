pub mod mcp;
pub mod model;
mod rates;
pub mod store;
use model::*;
use serde_json::{json, Value};
use std::path::PathBuf;
use store::Store;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};

pub fn catalog() -> Value {
    serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/catalog.json")))
        .expect("validated embedded catalog")
}
#[tauri::command]
fn snapshot(currency: Option<String>, period: Option<String>) -> Result<Value> {
    let (db, path) = Store::system()?.read()?;
    let summary = db.summary(
        currency.as_deref().unwrap_or(&db.settings.currency),
        period.as_deref().unwrap_or(&db.settings.period),
        today(),
    )?;
    let cny = db.summary("CNY", "month", today())?;
    let value = cny[&db.settings.level_basis]
        .as_str()
        .and_then(|s| decimal(s).ok());
    let level = if db.settings.show_level {
        value.and_then(|v| {
            db.settings
                .levels
                .iter()
                .filter(|l| decimal(&l.above).is_ok_and(|n| v > n))
                .max_by_key(|l| decimal(&l.above).unwrap_or_default())
                .map(|l| l.name.clone())
        })
    } else {
        None
    };
    Ok(
        json!({"database":db,"path":path,"summary":summary,"level":level,"executable":std::env::current_exe().map_err(|e|e.to_string())?}),
    )
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
fn show_main(app: &tauri::AppHandle, action: &str) -> tauri::Result<()> {
    if let Some(panel) = app.get_webview_window("panel") {
        let _ = panel.destroy();
    }
    if let Some(window) = app.get_webview_window("main") {
        window.show()?;
        window.set_focus()?;
        if ["settings", "add"].contains(&action) {
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
                _ => "index.html",
            }
            .into(),
        ),
    )
    .title("Subbit")
    .inner_size(880., 720.)
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
            .inner_size(400., 590.)
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
        let height = 590. * scale;
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
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            let _ = show_main(app, "");
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
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
            quit
        ])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "打开 Subbit / Open", true, None::<&str>)?;
            let settings =
                MenuItem::with_id(app, "settings", "设置 / Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 / Quit", true, None::<&str>)?;
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
