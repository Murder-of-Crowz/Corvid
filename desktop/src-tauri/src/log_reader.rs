//! Follows the newest WoW: Forever combat log and reads new lines as the game
//! writes them. Call [`LogFollower::poll`] on a timer; each call returns the
//! lines finished since the last one.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Combat log names start with this. Forever adds a timestamp before ".txt".
const LOG_PREFIX: &str = "WoWCombatLog";

pub struct LogFollower {
    folder: PathBuf,
    current: Option<OpenLog>,
}

impl LogFollower {
    /// Follows combat logs in `folder`, usually the client's `Logs` folder.
    /// The folder doesn't have to exist yet.
    pub fn new(folder: impl Into<PathBuf>) -> Self {
        Self {
            folder: folder.into(),
            current: None,
        }
    }

    /// The log being followed, once one has been found.
    pub fn current_path(&self) -> Option<&Path> {
        self.current.as_ref().map(|log| log.path.as_path())
    }

    /// Returns the lines finished since the last call, oldest first.
    ///
    /// The first log found is read from its end, so fights from before the
    /// app started aren't replayed. When the game starts a newer log, the
    /// rest of the old one is read first, then the new one from its start.
    pub fn poll(&mut self) -> io::Result<Vec<String>> {
        let newest = newest_log(&self.folder)?;
        let mut lines = Vec::new();

        match (&mut self.current, newest) {
            (None, None) => {}
            (None, Some(path)) => self.current = Some(OpenLog::at_end(path)?),
            (Some(log), Some(path)) if log.path != path => {
                log.read_lines(&mut lines)?;
                let mut next = OpenLog::at_start(path);
                next.read_lines(&mut lines)?;
                self.current = Some(next);
            }
            (Some(log), _) => log.read_lines(&mut lines)?,
        }
        Ok(lines)
    }
}

/// Every combat log in the folder, the one written to most recently first.
/// A missing folder has none.
pub fn combat_logs(folder: &Path) -> io::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };

    let mut logs = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let is_log = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(LOG_PREFIX) && name.ends_with(".txt"));
        if !is_log {
            continue;
        }
        // Ask the file itself: on Windows, the times in a directory listing can
        // lag behind while the game holds the file open. Skip the files that vanish.
        let Ok(modified) = fs::metadata(&path).and_then(|meta| meta.modified()) else {
            continue;
        };
        logs.push((modified, path));
    }
    logs.sort_by(|a, b| b.cmp(a));
    Ok(logs.into_iter().map(|(_, path)| path).collect())
}

// The combat log the game wrote to most recently, if the folder has any.
pub fn newest_log(folder: &Path) -> io::Result<Option<PathBuf>> {
    Ok(combat_logs(folder)?.into_iter().next())
}

struct OpenLog {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
    skip_to_newline: bool,
}

impl OpenLog {
    fn at_start(path: PathBuf) -> Self {
        Self {
            path,
            offset: 0,
            partial: Vec::new(),
            skip_to_newline: false,
        }
    }

    fn at_end(path: PathBuf) -> io::Result<Self> {
        let mut file = File::open(&path)?;
        let len = file.metadata()?.len();
        let mut mid_line = false;
        if len > 0 {
            file.seek(SeekFrom::Start(len - 1))?;
            let mut last = [0u8; 1];
            file.read_exact(&mut last)?;
            mid_line = last[0] != b'\n';
        }
        Ok(Self {
            path,
            offset: len,
            partial: Vec::new(),
            skip_to_newline: mid_line,
        })
    }

    /// Appends every line finished since the last read to `lines`.
    fn read_lines(&mut self, lines: &mut Vec<String>) -> io::Result<()> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };

        let len = file.metadata()?.len();
        if len < self.offset {
            *self = Self::at_start(std::mem::take(&mut self.path));
        }
        if len == self.offset {
            return Ok(());
        }

        file.seek(SeekFrom::Start(self.offset))?;
        let read = file.read_to_end(&mut self.partial)?;
        self.offset += read as u64;

        let data = std::mem::take(&mut self.partial);
        let mut rest = data.as_slice();
        if self.skip_to_newline {
            let Some(end) = rest.iter().position(|&byte| byte == b'\n') else {
                return Ok(());
            };
            rest = &rest[end + 1..];
            self.skip_to_newline = false;
        }
        while let Some(end) = rest.iter().position(|&byte| byte == b'\n') {
            let line = &rest[..end];
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if !line.is_empty() {
                lines.push(String::from_utf8_lossy(line).into_owned());
            }
            rest = &rest[end + 1..];
        }
        self.partial = rest.to_vec();
        Ok(())
    }
}
