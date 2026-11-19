//! Draws a map from a real World of Warcraft install. It needs the game, so
//! it's skipped unless asked for, on a computer that has it.
//! From `desktop/src-tauri/`, in PowerShell:
//!
//! ```powershell
//! $env:CORVID_WOW_DIR = "C:\path\to\World of Warcraft"
//! cargo test --test maps -- --ignored
//! ```

use desktop_lib::{casc, maps, wow};
use std::fs;
use std::path::PathBuf;

/// A folder under the system temp directory, removed when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("corvid-maps-{}-{test}", std::process::id()));
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

/// Tirisfal Glades, by the uiMapID the combat log uses.
const TIRISFAL_GLADES: u32 = 1420;

#[test]
#[ignore = "needs the game: set CORVID_WOW_DIR and pass --ignored"]
fn draws_a_zone_from_the_game_and_caches_it() {
    let wow = PathBuf::from(
        std::env::var("CORVID_WOW_DIR")
            .expect("set CORVID_WOW_DIR to the World of Warcraft folder"),
    );
    let flavor = wow::flavor(&wow);
    let cache = TempDir::new("draw");

    let png = maps::png(&flavor, &cache.0, TIRISFAL_GLADES)
        .unwrap()
        .expect("the game has a map for Tirisfal Glades");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    // A PNG's width and height come first in its header chunk.
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert_eq!((width, height), (1002, 668));

    // It's kept for this version of the game, and comes from there next time.
    let folder = cache.0.join(casc::version(&flavor).unwrap());
    assert!(folder.join("1420.png").is_file());
    assert_eq!(
        maps::png(&flavor, &cache.0, TIRISFAL_GLADES).unwrap(),
        Some(png)
    );

    // A uiMapID the game has no map for is remembered as an empty file.
    assert_eq!(maps::png(&flavor, &cache.0, 999_999).unwrap(), None);
    assert_eq!(fs::read(folder.join("999999.png")).unwrap(), b"");
}
