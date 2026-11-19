//! Tests that settings survive a save and load, and that a missing or broken
//! file falls back to the defaults instead of stopping the app.

use desktop_lib::settings::Settings;
use std::fs;
use std::path::PathBuf;

/// A folder under the system temp directory, removed when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("corvid-settings-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_missing_file_gives_the_defaults() {
    let dir = TempDir::new("missing");
    assert_eq!(
        Settings::load(&dir.0.join("settings.json")),
        Settings::default()
    );
    assert_eq!(Settings::default().wow_dir, None);
}

#[test]
fn saves_and_loads_the_wow_folder() {
    let dir = TempDir::new("roundtrip");
    // The app config folder may not exist yet on first run.
    let path = dir.0.join("not-yet").join("settings.json");
    let settings = Settings {
        wow_dir: Some(PathBuf::from(r"D:\Games\World of Warcraft")),
    };

    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path), settings);

    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        json,
        serde_json::json!({ "wowDir": r"D:\Games\World of Warcraft" })
    );
}

#[test]
fn a_broken_file_gives_the_defaults() {
    let dir = TempDir::new("broken");
    let path = dir.0.join("settings.json");
    fs::write(&path, "{ not json").unwrap();
    assert_eq!(Settings::load(&path), Settings::default());
}

#[test]
fn missing_and_unknown_fields_are_fine() {
    let dir = TempDir::new("fields");
    let path = dir.0.join("settings.json");
    fs::write(&path, r#"{ "fromANewerVersion": true }"#).unwrap();
    assert_eq!(Settings::load(&path), Settings::default());
}
