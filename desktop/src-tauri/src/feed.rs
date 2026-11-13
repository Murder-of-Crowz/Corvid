//! Follows the combat log while the app runs, parses each new line, and sends
//! the events to the UI.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Runtime};

use crate::events::{CorvidEvent, EVENTS_CHANNEL, Timestamp};
use crate::log_reader::LogFollower;
use crate::parser::Parser;

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
    let mut parser = Parser::new();
    let mut following: Option<PathBuf> = None;
    let mut last_error: Option<String> = None;

    loop {
        if let Some(folder) = chosen.try_iter().last() {
            follower = LogFollower::new(folder);
        }

        let mut events = Vec::new();
        match follower.poll() {
            Ok(lines) => {
                last_error = None;
                for line in &lines {
                    parser.parse_line(line, &mut events)
                }
            }
            Err(error) => {
                let message = error.to_string();
                if last_error.as_ref() != Some(&message) {
                    eprintln!("[corvid] couldn't read the combat log: {message}");
                }
                last_error = Some(message);
            }
        }
        parser.end_idle_fight(now_ms(), &mut events);
        send(&app, events);

        if follower.current_path() != following.as_deref() {
            following = follower.current_path().map(Path::to_path_buf);
            if let Some(path) = &following {
                println!("[corvid] following {}", path.display());
                catch_up(&mut parser, path)
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
}


/// Reads the zone and map from what `path` already holds. The game only writes
/// them when a log starts or the player changes zone, and the follower starts
/// at the end of a log that's already open.
fn catch_up(parser: &mut Parser, path: &Path) {
    match fs::read(path) {
        Ok(bytes) => {
            for line in String::from_utf8_lossy(&bytes).lines() {
                parser.read_zone_and_map(line);
            }
        }
        Err(error) => eprintln!("[corvid] couldn't read {}: {error}", path.display()),
    }
}

fn send<R: Runtime>(app: &AppHandle<R>, events: Vec<CorvidEvent>) {
    if events.is_empty() {
        return;
    }
    if let Err(error) = app.emit(EVENTS_CHANNEL, events) {
        eprintln!("[corvid] couldn't send events to the UI: {error}");
    }
}

fn now_ms() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as Timestamp)
}
