pub mod blp;
pub mod casc;
pub mod db2;
pub mod events;
pub mod feed;
pub mod log_reader;
pub mod maps;
pub mod parser;
pub mod settings;
pub mod strip;
pub mod wow;

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::thread;

use tauri::http::{Response, header::CONTENT_TYPE};
use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use events::CorvidEvent;
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
fn wow_dir(state: State<'_, AppState>) -> Option<PathBuf> {
    state.settings.lock().unwrap().wow_dir.clone()
}

fn logs_dir(state: State<'_, AppState>) -> Option<PathBuf> {
    wow_dir(state).map(|wow| wow::logs(&wow))
}

/// Asks the player for the combat log folder, then saves it and follows it.
/// Returns `null` if they cancel.
#[tauri::command]
async fn choose_wow_dir(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<PathBuf>, String> {
    let mut picker = app
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Choose your World of Warcraft folder");
    if let Some(current) = wow_dir(state.clone()) {
        picker = picker.set_directory(current);
    }
    // Async commands run off the main thread, so waiting here is fine.
    let Some(picked) = picker.blocking_pick_folder() else {
        return Ok(None);
    };
    let picked = picked.into_path().map_err(|error| error.to_string())?;

    let Some(folder) = wow::find(&picked) else {
        app.dialog()
            .message(format!(
                "There's no {} folder in {}. Choose the World of Warcraft folder the game is installed in.",
                wow::FLAVOR,
                picked.display()
            ))
            .kind(MessageDialogKind::Error)
            .parent(&window)
            .blocking_show();
        return Ok(None);
    };

    let mut settings = state.settings.lock().unwrap();
    settings.wow_dir = Some(folder.clone());
    settings
        .save(&state.settings_path)
        .map_err(|error| format!("couldn't save the settings: {error}"))?;
    state.feed.set_folder(wow::logs(&folder));
    Ok(Some(folder))
}

#[tauri::command]
fn list_logs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let Some(folder) = logs_dir(state) else {
        return Ok(Vec::new());
    };
    let logs = log_reader::combat_logs(&folder).map_err(|error| error.to_string())?;
    Ok(logs
        .iter()
        .filter_map(|path| Some(path.file_name()?.to_str()?.to_string()))
        .collect())
}

#[tauri::command]
async fn read_log(name: String, state: State<'_, AppState>) -> Result<Vec<CorvidEvent>, String> {
    let folder = logs_dir(state).ok_or("no World of Warcraft folder has been chosen")?;
    let path = log_reader::combat_logs(&folder)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|path| path.file_name().is_some_and(|file| file == name.as_str()))
        .ok_or_else(|| format!("{name} isn't a combat log in {}", folder.display()))?;
    let bytes = fs::read(&path).map_err(|error| format!("couldn't read {name}: {error}"))?;
    Ok(parser::parse_log(&String::from_utf8_lossy(&bytes)))
}

fn serve_map(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let id = path
        .trim_start_matches('/')
        .strip_suffix(".png")
        .and_then(|id| id.parse().ok());
    match id.and_then(|id| map_png(app, id)) {
        Some(png) => Response::builder()
            .header(CONTENT_TYPE, "image/png")
            .body(png),
        None => Response::builder().status(404).body(Vec::new()),
    }
    .unwrap()
}

fn map_png(app: &AppHandle, id: u32) -> Option<Vec<u8>> {
    let wow = app
        .state::<AppState>()
        .settings
        .lock()
        .unwrap()
        .wow_dir
        .clone()?;
    maps::png(&wow::flavor(&wow), &wow::maps(&wow), id).unwrap_or_else(|error| {
        eprintln!("[corvid] couldn't draw map {id}: {error}");
        None
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if let Ok(cache) = app.path().app_cache_dir() {
                let _ = fs::remove_dir_all(cache.join("Maps"));
            }
            let settings_path = app.path().app_config_dir()?.join("settings.json");
            let settings = Settings::load(&settings_path);
            let feed = Feed::start(app.handle().clone(), settings.wow_dir.as_deref().map(wow::logs));
            app.manage(AppState {
                settings_path,
                settings: Mutex::new(settings),
                feed,
            });
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("map", |context, request, responder| {
            let app = context.app_handle().clone();
            let path = request.uri().path().to_string();
            thread::spawn(move || responder.respond(serve_map(&app, &path)));
        })
        .invoke_handler(tauri::generate_handler![
            wow_dir,
            choose_wow_dir,
            list_logs,
            read_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
