pub mod events;
pub mod feed;
pub mod log_reader;
pub mod parser;
pub mod settings;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use feed::Feed;
use settings::Settings;

/// What the commands share while the app runs.
struct AppState {
    settings_path: PathBuf,
    settings: Mutex<Settings>,
    feed: Feed,
}

/// The combat log folder, or `null` until the player chooses one.
#[tauri::command]
fn logs_dir(state: State<'_, AppState>) -> Option<PathBuf> {
    state.settings.lock().unwrap().logs_dir.clone()
}

/// Asks the player for the combat log folder, then saves it and follows it.
/// Returns `null` if they cancel.
#[tauri::command]
async fn choose_logs_dir(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<PathBuf>, String> {
    let mut picker = app
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Choose the folder with your WoWCombatLog files");
    if let Some(current) = logs_dir(state.clone()) {
        picker = picker.set_directory(current);
    }
    // Async commands run off the main thread, so waiting here is fine.
    let Some(picked) = picker.blocking_pick_folder() else {
        return Ok(None);
    };
    let folder = picked.into_path().map_err(|error| error.to_string())?;

    let mut settings = state.settings.lock().unwrap();
    settings.logs_dir = Some(folder.clone());
    settings
        .save(&state.settings_path)
        .map_err(|error| format!("couldn't save the settings: {error}"))?;
    state.feed.set_folder(folder.clone());
    Ok(Some(folder))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = Settings::load(&settings_path);
            let feed = Feed::start(app.handle().clone(), settings.logs_dir.clone());
            app.manage(AppState {
                settings_path,
                settings: Mutex::new(settings),
                feed,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![logs_dir, choose_logs_dir])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
