//! What the last precompile saw, so that a launch need not look again when
//! nothing changed.
//!
//! Walking the title reads every one of its files to find the shaders in
//! them: about a minute on this install, the same minute at every launch.
//! So a precompile writes down what it saw -- the build, the `VICTORY_*`
//! switches a translation depends on, the title's files by their count and
//! newest change, and how many shaders the cache held -- and `--if-needed`
//! reads it back: the same, and the walk is skipped.
//!
//! Nothing is lost when this says "the same" and something did change: the
//! run translates the shaders it needs at their first use, as it did before
//! precompiling existed (with the hitch precompiling avoids). What must not
//! happen is the other way round, a walk at every launch, so what is written
//! is only what is cheap to read: a file's length and time, and a count of
//! names. The pipeline book and the host's library change at every run and
//! are left out: the pipelines are warmed at every launch either way, and
//! `warm` translates and makes those it has not made yet.
//!
//! The cache only ever gains entries -- a run translates during play -- so
//! one that holds at least what was written down is still the one it saw;
//! only a cache that lost entries (wiped, or not the one it was) is walked
//! again.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

/// What was written; a state of another shape is not read.
const VERSION: &str = "victory precompile state 1";

/// Everything a walk of the title depends on, as one text to write down.
pub fn of(shaders: &Path, paths: &[PathBuf]) -> String {
    let mut text = String::from(VERSION);
    text.push('\n');
    text.push_str(&this_build());
    text.push_str(&switches());
    for root in paths {
        text.push_str(&title(root));
    }
    text.push_str(&format!("shaders {}\n", cached(shaders)));
    text
}

/// The executable: a new build translates differently.
fn this_build() -> String {
    let Ok(exe) = std::env::current_exe() else {
        return "build unknown\n".to_string();
    };
    let Ok(meta) = std::fs::metadata(&exe) else {
        return format!("build {} unknown\n", exe.display());
    };
    format!(
        "build {} {} {}\n",
        exe.display(),
        meta.len(),
        ticks(meta.modified().ok())
    )
}

/// The `VICTORY_*` switches of this process, in order: a translation depends
/// on them (`shader_cache::build`).
fn switches() -> String {
    let mut vars: Vec<(String, String)> = std::env::vars()
        .filter(|(name, _)| name.starts_with("VICTORY_"))
        .collect();
    vars.sort();
    vars.iter()
        .map(|(name, value)| format!("switch {name}={value}\n"))
        .collect()
}

/// The files of one root, without reading them: how many, and the newest
/// change of them all.
fn title(root: &Path) -> String {
    let mut files = 0u64;
    let mut newest = 0u128;
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        files += 1;
        let changed = entry.metadata().ok().and_then(|m| m.modified().ok());
        newest = newest.max(ticks(changed));
    }
    format!("title {} {files} {newest}\n", root.display())
}

/// How many shaders the cache holds: a wiped cache, or one given entries by
/// `--load`, has them translated again.
fn cached(shaders: &Path) -> u64 {
    let mut n = 0u64;
    let walk = walkdir::WalkDir::new(shaders).max_depth(2);
    for entry in walk.into_iter().filter_map(Result::ok) {
        if entry.depth() == 2 && entry.file_type().is_file() {
            n += 1;
        }
    }
    n
}

fn ticks(time: Option<SystemTime>) -> u128 {
    time.and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos())
}

/// A state read back.
#[derive(Debug)]
struct Seen {
    /// The `build`, `switch` and `title` lines, in the order they were written.
    lines: Vec<String>,
    /// How many shaders the cache held.
    shaders: u64,
}

impl Seen {
    fn parse(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()? != VERSION {
            return None;
        }
        let mut seen = Seen {
            lines: Vec::new(),
            shaders: 0,
        };
        for line in lines {
            match line.strip_prefix("shaders ") {
                Some(n) => seen.shaders = n.parse().ok()?,
                None => seen.lines.push(line.to_string()),
            }
        }
        Some(seen)
    }

    /// This one, when the cache is still one that holds its shaders: entries are
    /// only ever added, so more of them than were written down is not a change.
    fn covers(&self, now: &Seen) -> bool {
        self.lines == now.lines && now.shaders >= self.shaders
    }
}

/// Whether the last precompile saw this, and left a file saying so.
pub fn unchanged(file: &Path, now: &str) -> bool {
    let Some(now) = Seen::parse(now) else {
        return false;
    };
    std::fs::read_to_string(file)
        .ok()
        .and_then(|was| Seen::parse(&was))
        .is_some_and(|was| was.covers(&now))
}

/// Keeps what this one saw, for the next launch.
pub fn record(file: &Path, now: &str) -> Result<()> {
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let staging = file.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&staging, now)?;
    std::fs::rename(&staging, file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("victory-state-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn entry(cache: &Path) -> PathBuf {
        let file = cache.join("ab").join("a".repeat(62));
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "one").unwrap();
        file
    }

    #[test]
    fn a_title_that_gained_a_file_is_not_what_was_seen() {
        let dir = folder("title");
        let game = dir.join("game");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("a"), "a").unwrap();
        let cache = dir.join("shaders");
        let before = of(&cache, std::slice::from_ref(&game));
        assert_eq!(before, of(&cache, std::slice::from_ref(&game)));
        std::fs::write(game.join("b"), "b").unwrap();
        assert_ne!(before, of(&cache, std::slice::from_ref(&game)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cache_that_lost_a_shader_is_not_what_was_seen() {
        let dir = folder("lost");
        let game = dir.join("game");
        std::fs::create_dir_all(&game).unwrap();
        let file = dir.join("precompiled.state");
        let cache = dir.join("shaders");
        let shader = entry(&cache);
        record(&file, &of(&cache, std::slice::from_ref(&game))).unwrap();
        std::fs::remove_file(shader).unwrap();
        assert!(!unchanged(&file, &of(&cache, std::slice::from_ref(&game))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cache_that_gained_shaders_is_still_what_was_seen() {
        let dir = folder("gained");
        let game = dir.join("game");
        std::fs::create_dir_all(&game).unwrap();
        let file = dir.join("precompiled.state");
        let cache = dir.join("shaders");
        record(&file, &of(&cache, std::slice::from_ref(&game))).unwrap();
        entry(&cache);
        assert!(unchanged(&file, &of(&cache, std::slice::from_ref(&game))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn what_was_recorded_is_read_back() {
        let dir = folder("record");
        let file = dir.join("precompiled.state");
        let game = dir.join("game");
        std::fs::create_dir_all(&game).unwrap();
        let now = of(&dir.join("shaders"), std::slice::from_ref(&game));
        assert!(!unchanged(&file, &now));
        record(&file, &now).unwrap();
        assert!(unchanged(&file, &now));
        assert!(!unchanged(&file, "victory precompile state 1\nshaders 0\n"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
