//! Tests the combat log follower on real files in a temporary folder.

use desktop_lib::log_reader::{LogFollower, combat_logs, newest_log};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// A folder under the system temp directory, removed when the test ends.
struct TempDir(PathBuf);

impl TempDir {
    fn new(test: &str) -> Self {
        let path = std::env::temp_dir().join(format!("corvid-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
}

fn append(path: &Path, text: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// Sets a file's modified time, so tests don't depend on how fast they run.
fn set_modified(path: &Path, time: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

#[test]
fn finds_nothing_in_a_missing_or_empty_folder() {
    let dir = TempDir::new("empty");
    let mut missing = LogFollower::new(dir.file("not-there"));
    assert!(missing.poll().unwrap().is_empty());

    let mut empty = LogFollower::new(&dir.0);
    assert!(empty.poll().unwrap().is_empty());
    assert!(empty.current_path().is_none());
}

#[test]
fn starts_at_the_end_then_reads_new_lines() {
    let dir = TempDir::new("end");
    let log = dir.file("WoWCombatLog-110526_231729.txt");
    write(&log, "old 1\nold 2\n");

    let mut follower = LogFollower::new(&dir.0);
    assert!(
        follower.poll().unwrap().is_empty(),
        "old lines aren't replayed"
    );
    assert_eq!(follower.current_path(), Some(log.as_path()));

    append(&log, "new 1\nnew 2\n");
    assert_eq!(follower.poll().unwrap(), ["new 1", "new 2"]);
    assert!(follower.poll().unwrap().is_empty());
}

#[test]
fn waits_for_the_end_of_a_line() {
    let dir = TempDir::new("partial");
    let log = dir.file("WoWCombatLog-110526_231729.txt");
    write(&log, "");
    let mut follower = LogFollower::new(&dir.0);
    follower.poll().unwrap();

    append(&log, "SPELL_DAMAGE,half");
    assert!(follower.poll().unwrap().is_empty());
    append(&log, " and the rest\n");
    assert_eq!(follower.poll().unwrap(), ["SPELL_DAMAGE,half and the rest"]);
}

#[test]
fn strips_windows_line_endings_and_keeps_accents() {
    let dir = TempDir::new("utf8");
    let log = dir.file("WoWCombatLog-110526_231729.txt");
    write(&log, "");
    let mut follower = LogFollower::new(&dir.0);
    follower.poll().unwrap();

    append(&log, "\"Zoë-Ångström-Normal-US\"\r\nnext\r\n");
    assert_eq!(
        follower.poll().unwrap(),
        ["\"Zoë-Ångström-Normal-US\"", "next"]
    );
}

#[test]
fn drops_the_line_it_joined_halfway_through() {
    let dir = TempDir::new("midline");
    let log = dir.file("WoWCombatLog-110526_231729.txt");
    write(&log, "old\nhalf of a li");

    let mut follower = LogFollower::new(&dir.0);
    follower.poll().unwrap();
    append(&log, "ne\nwhole line\n");
    assert_eq!(follower.poll().unwrap(), ["whole line"]);
}

#[test]
fn moves_to_a_newer_log_without_losing_lines() {
    let dir = TempDir::new("rotate");
    let now = SystemTime::now();
    let old = dir.file("WoWCombatLog-110526_231729.txt");
    write(&old, "a0\n");
    set_modified(&old, now - Duration::from_secs(60));

    let mut follower = LogFollower::new(&dir.0);
    follower.poll().unwrap();
    append(&old, "a1\n");

    let new = dir.file("WoWCombatLog-110626_060352.txt");
    write(&new, "b1\nb2\n");
    set_modified(&new, now + Duration::from_secs(60));

    assert_eq!(follower.poll().unwrap(), ["a1", "b1", "b2"]);
    assert_eq!(follower.current_path(), Some(new.as_path()));
}

#[test]
fn starts_over_when_the_file_shrinks() {
    let dir = TempDir::new("shrink");
    let log = dir.file("WoWCombatLog-110526_231729.txt");
    write(&log, "1\n2\n3\n");
    let mut follower = LogFollower::new(&dir.0);
    follower.poll().unwrap();

    write(&log, "x\n");
    assert_eq!(follower.poll().unwrap(), ["x"]);
}

#[test]
fn only_counts_combat_logs() {
    let dir = TempDir::new("others");
    let now = SystemTime::now();
    let combat = dir.file("WoWCombatLog-110526_231729.txt");
    write(&combat, "x\n");
    set_modified(&combat, now - Duration::from_secs(60));
    for other in ["WoWChatLog.txt", "notes.txt", "WoWCombatLog-backup.zip"] {
        write(&dir.file(other), "x\n");
        set_modified(&dir.file(other), now);
    }

    assert_eq!(newest_log(&dir.0).unwrap(), Some(combat));
}

#[test]
fn lists_every_combat_log_newest_first() {
    let dir = TempDir::new("list");
    let start = SystemTime::now() - Duration::from_secs(600);
    for (minutes, name) in [
        (1, "WoWCombatLog-110526_231729.txt"),
        (3, "WoWCombatLog-110726_214017.txt"),
        (2, "WoWCombatLog-110626_060352.txt"),
        (4, "WoWChatLog.txt"),
    ] {
        write(&dir.file(name), "x\n");
        set_modified(&dir.file(name), start + Duration::from_secs(60 * minutes));
    }

    let names: Vec<_> = combat_logs(&dir.0)
        .unwrap()
        .iter()
        .map(|path| path.file_name().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        [
            "WoWCombatLog-110726_214017.txt",
            "WoWCombatLog-110626_060352.txt",
            "WoWCombatLog-110526_231729.txt",
        ]
    );
    assert!(combat_logs(&dir.file("not-there")).unwrap().is_empty());
}
