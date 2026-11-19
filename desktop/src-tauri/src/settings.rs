//! Choices saved between runs, as JSON in the app's config folder.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub wow_dir: Option<PathBuf>
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str(&text).unwrap_or_else(|error| {
            eprintln!("[corvid] ignoring {}: {error}", path.display());
            Self::default()
        })
    }

    /// Write the settings to `path`, creating its folder if needed.
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        fs::write(path, serde_json::to_string_pretty(self)?)
    }
}
