//! Where Corvid finds things in the WoW folder

use std::path::{Path, PathBuf};

/// The game's own folder in the WoW folder, with its logs and addons.
pub const FLAVOR: &str = "_forever_";

/// The WoW folder `picked` is, or is inside, like when the player picks
/// `_forever_` or its `Logs` folder instead.
pub fn find(picked: &Path) -> Option<PathBuf> {
    picked
        .ancestors()
        .find(|folder| folder.join(FLAVOR).is_dir())
        .map(Path::to_path_buf)
}

pub fn flavor(wow: &Path) -> PathBuf {
    wow.join(FLAVOR)
}

/// Where the game writes it combat logs.
pub fn logs(wow: &Path) -> PathBuf {
    flavor(wow).join("Logs")
}