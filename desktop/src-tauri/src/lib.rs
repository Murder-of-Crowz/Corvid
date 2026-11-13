// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod events;
pub mod feed;
pub mod log_reader;

const LOGS_DIR_VAR: &str = "CORVID_LOGS_DIR";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            match std::env::var_os(LOGS_DIR_VAR) {
                Some(folder) => feed::start(app.handle().clone(), folder.into()),
                None => eprintln!("[corvid] {LOGS_DIR_VAR} isn't set, so no combat log is read.")
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}