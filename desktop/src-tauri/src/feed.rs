//! Sends new combat log lines to the UI while the app runs. For now each line
//! goes out as a `rawLine` event; the parser will turn them into real events.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Runtime};

use crate::events::{CorvidEvent, EVENTS_CHANNEL, LogKind, RawLineEvent, Timestamp};
use crate::log_reader::LogFollower;

// How often the log is checked for new lines.
const POLL_INTERVAL: Duration = Duration::from_millis(100);

pub struct Feed {
    folders: Sender<PathBuf>,
}

impl Feed {
    /// Follows the combat logs in `folder` on a background thread for as long
    /// as the app runs. With no folder, it waits until one is set.
    pub fn start<R: Runtime>(app: AppHandle<R>, folder: Option<PathBuf>) -> Self {
        let (folders, chosen) = mpsc::channel();
        thread::spawn(move || follow(app, folder, chosen));
        Self { folders }
    }

    /// Follows `folder` instead, from the end of its newest log.
    pub fn set_folder(&self, folder: PathBuf) {
        let _ = self.folders.send(folder);
    }
}

/// The feed thread. Each poll's new lines are sent as one batch.
fn follow<R: Runtime>(app: AppHandle<R>, folder: Option<PathBuf>, chosen: Receiver<PathBuf>) {
    let Some(folder) = folder.or_else(|| chosen.recv().ok()) else {
        return;
    };
    let mut follower = LogFollower::new(folder);
    let mut following: Option<PathBuf> = None;
    let mut last_error: Option<String> = None;

    loop {
        if let Some(folder) = chosen.try_iter().last() {
            follower = LogFollower::new(folder);
        }

        match follower.poll() {
            Ok(lines) => {
                last_error = None;
                send(&app, lines);
            }
            Err(error) => {
                let message = error.to_string();
                if last_error.as_ref() != Some(&message) {
                    eprintln!("[corvid] couldn't read the combat log: {message}");
                }
                last_error = Some(message);
            }
        }

        if follower.current_path() != following.as_deref() {
            following = follower.current_path().map(Path::to_path_buf);
            if let Some(path) = &following {
                println!("[corvid] following {}", path.display());
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
}

/// Emits `lines` as `rawLine` events. Their `ts` is when Corvid read
/// them; the game's own timestamp is still at the start of each line.
fn send<R: Runtime>(app: &AppHandle<R>, lines: Vec<String>) {
    if lines.is_empty() {
        return;
    }
    let ts = now_ms();
    let batch: Vec<CorvidEvent> = lines
        .into_iter()
        .map(|line| {
            CorvidEvent::RawLine(RawLineEvent {
                ts,
                log: LogKind::Combat,
                line,
            })
        })
        .collect();
    if let Err(error) = app.emit(EVENTS_CHANNEL, batch) {
        eprintln!("[corvid] couldn't send events to the UI: {error}");
    }
}

fn now_ms() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as Timestamp)
}
