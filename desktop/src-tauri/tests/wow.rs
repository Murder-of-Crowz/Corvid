//! Tests finding the World of Warcraft folder from whichever folder the player
//! picks, on real folders in a temporary directory.

use desktop_lib::wow;
use std::fs;
use std::path::PathBuf;

/// A folder under the system temp directory, removed when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("corvid-wow-{}-{test}", std::process::id()));
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

/// A WoW folder with the game's `_forever_\Logs` folder in it.
fn install(dir: &TempDir) -> PathBuf {
    let wow = dir.0.join("World of Warcraft");
    fs::create_dir_all(wow.join("_forever_").join("Logs")).unwrap();
    wow
}

#[test]
fn finds_the_wow_folder_when_its_picked() {
    let dir = TempDir::new("itself");
    let wow = install(&dir);
    assert_eq!(wow::find(&wow), Some(wow));
}

#[test]
fn finds_the_wow_folder_from_a_folder_inside_it() {
    let dir = TempDir::new("inside");
    let wow = install(&dir);
    assert_eq!(wow::find(&wow.join("_forever_")), Some(wow.clone()));
    assert_eq!(wow::find(&wow.join("_forever_").join("Logs")), Some(wow));
}

#[test]
fn a_folder_without_the_game_isnt_the_wow_folder() {
    let dir = TempDir::new("without");
    install(&dir);
    let downloads = dir.0.join("Downloads");
    fs::create_dir_all(&downloads).unwrap();

    // The folder above the WoW folder doesn't have _forever_ in it either.
    assert_eq!(wow::find(&dir.0), None);
    assert_eq!(wow::find(&downloads), None);
}

#[test]
fn a_file_named_like_the_game_folder_doesnt_count() {
    let dir = TempDir::new("file");
    fs::write(dir.0.join("_forever_"), "").unwrap();
    assert_eq!(wow::find(&dir.0), None);
}

#[test]
fn the_logs_are_in_the_game_folder() {
    let wow = PathBuf::from(r"D:\Games\World of Warcraft");
    assert_eq!(wow::flavor(&wow), wow.join("_forever_"));
    assert_eq!(wow::logs(&wow), wow.join("_forever_").join("Logs"));
}
