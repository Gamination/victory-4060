//! Where the user's files live.
//!
//! Resolution order for each value: environment variable, then the config
//! file (`VICTORY_CONFIG`, else `./victory.toml`), then the default.
//! See `victory.example.toml` at the repository root.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::policy::ensure_outside_repo;

#[derive(Debug, Default, Clone)]
pub struct Config {
    /// The user's dump of the title.
    pub game_dir: Option<PathBuf>,
    /// Root for all derived data. Always outside the repository.
    pub cache_dir: PathBuf,
    /// What titles write and must keep (persistent local storage). Never
    /// inside the cache, which may be deleted at any time.
    pub data_dir: PathBuf,
    /// Other titles used to exercise platform-level code.
    pub test_titles: Vec<PathBuf>,
    /// The config file that was read, if any.
    pub source: Option<PathBuf>,
    /// Memory configuration settings.
    pub memory: MemoryConfig,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryConfig {
    /// Enable low-RAM optimizations for systems with 16GB or less RAM.
    /// If not set, auto-detects based on physical RAM.
    pub low_ram: Option<bool>,
    /// Arena size in MiB for title memory outside buffers.
    pub arena_mb: Option<u64>,
    /// Maximum pages of title memory to keep (in 64 KiB pages).
    pub max_pages: Option<usize>,
    /// Maximum page faults to handle per frame.
    pub faults_per_frame: Option<usize>,
    /// Maximum total page faults to handle across all frames.
    pub faults_all: Option<usize>,
    /// Upload arena size in MiB for shader-accessible memory.
    pub upload_arena_mb: Option<usize>,
    /// Texture pool: maximum spare blocks to keep (each 64 MiB).
    pub texture_spare_kept: Option<usize>,
    /// Staging pool: blocks of upload buffers to keep.
    pub staging_kept: Option<usize>,
    /// Ripe blocks: blocks made ahead of time to avoid allocation stalls.
    pub ripe_blocks: Option<usize>,
    /// Card buffer limits: minimum buffer size in KiB to place in VRAM.
    pub card_buffers_min_kb: Option<u64>,
    /// Card buffer limits: maximum VRAM for card buffer copies in MiB.
    pub card_buffers_mb: Option<u64>,
}

impl MemoryConfig {
    fn with_environment(mut self, env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Self {
        fn numeric<T: std::str::FromStr>(value: Option<String>, fallback: Option<T>) -> Option<T> {
            value.and_then(|v| v.trim().parse().ok()).or(fallback)
        }
        let text = |name: &str| env(name).and_then(|v| v.into_string().ok());
        if let Some(value) = text("VICTORY_LOW_RAM") {
            match value.trim().to_ascii_lowercase().as_str() {
                "1" | "true" => self.low_ram = Some(true),
                "0" | "false" => self.low_ram = Some(false),
                _ => {}
            }
        }
        self.arena_mb = numeric(text("VICTORY_ARENA_MB"), self.arena_mb);
        self.max_pages = numeric(text("VICTORY_MAX_PAGES"), self.max_pages);
        self.faults_per_frame = numeric(text("VICTORY_FAULTS_PER_FRAME"), self.faults_per_frame);
        self.faults_all = numeric(text("VICTORY_FAULTS_ALL"), self.faults_all);
        self.upload_arena_mb = numeric(text("VICTORY_UPLOAD_ARENA_MB"), self.upload_arena_mb);
        self.texture_spare_kept =
            numeric(text("VICTORY_TEXTURE_SPARE_KEPT"), self.texture_spare_kept);
        self.staging_kept = numeric(text("VICTORY_STAGING_KEPT"), self.staging_kept);
        self.ripe_blocks = numeric(text("VICTORY_RIPE_BLOCKS"), self.ripe_blocks);
        self.card_buffers_min_kb = numeric(
            text("VICTORY_CARD_BUFFERS_MIN_KB"),
            self.card_buffers_min_kb,
        );
        self.card_buffers_mb = numeric(text("VICTORY_CARD_BUFFERS_MB"), self.card_buffers_mb);
        self
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    #[serde(default)]
    paths: FilePaths,
    #[serde(default)]
    memory: MemoryConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilePaths {
    game_dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    #[serde(default)]
    test_titles: Vec<PathBuf>,
}

impl Config {
    /// Loads configuration from the environment and the config file.
    pub fn load() -> Result<Self> {
        let explicit = explicit_config_path(|k| std::env::var_os(k));
        let path = explicit
            .clone()
            .unwrap_or_else(|| PathBuf::from("victory.toml"));
        let file = if path.is_file() {
            let text = std::fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            Some((path, text))
        } else if let Some(p) = explicit {
            anyhow::bail!(
                "VICTORY_CONFIG points at {}, which does not exist",
                p.display()
            );
        } else {
            None
        };
        let (source, parsed) = match file {
            Some((path, text)) => {
                let parsed: FileConfig =
                    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
                (Some(path), parsed)
            }
            None => (None, FileConfig::default()),
        };
        Self::resolve(parsed, source, |k| std::env::var_os(k))
    }

    fn resolve(
        file: FileConfig,
        source: Option<PathBuf>,
        env: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> Result<Self> {
        let env_path = |k: &str| env(k).filter(|v| !v.is_empty()).map(PathBuf::from);
        let test_titles = match env("VICTORY_TEST_TITLES") {
            Some(v) if !v.is_empty() => std::env::split_paths(&v)
                .filter(|p| !p.as_os_str().is_empty())
                .collect(),
            _ => file.paths.test_titles,
        };
        let cache_dir = env_path("VICTORY_CACHE_DIR")
            .or(file.paths.cache_dir)
            .map_or_else(|| default_dir("cache"), Ok)?;
        let data_dir = env_path("VICTORY_DATA_DIR")
            .or(file.paths.data_dir)
            .map_or_else(|| default_dir("data"), Ok)?;
        Ok(Config {
            game_dir: env_path("VICTORY_GAME_DIR").or(file.paths.game_dir),
            cache_dir,
            data_dir,
            test_titles,
            source,
            memory: file.memory.with_environment(env),
        })
    }

    /// The game directory, or a helpful error if it is not configured.
    pub fn require_game_dir(&self) -> Result<&Path> {
        self.game_dir.as_deref().context(
            "no game directory configured: set VICTORY_GAME_DIR or [paths].game_dir in \
             victory.toml (see victory.example.toml)",
        )
    }

    /// Returns `<cache>/<sub>`, creating it, after checking the asset policy.
    pub fn cache_subdir(&self, sub: &str) -> Result<PathBuf> {
        subdir(&self.cache_dir, sub)
    }

    /// Returns `<data>/<sub>`, creating it, after checking the asset policy.
    pub fn data_subdir(&self, sub: &str) -> Result<PathBuf> {
        subdir(&self.data_dir, sub)
    }
}

/// The config file explicitly named by the environment, if any. An empty
/// `VICTORY_CONFIG` means unset, like the other path variables.
fn explicit_config_path(env: impl Fn(&str) -> Option<std::ffi::OsString>) -> Option<PathBuf> {
    env("VICTORY_CONFIG")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

fn subdir(root: &Path, sub: &str) -> Result<PathBuf> {
    let dir = root.join(sub);
    ensure_outside_repo(&dir)?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir)
}

/// `%LOCALAPPDATA%/victory/<name>` (elsewhere the XDG or home cache).
fn default_dir(name: &str) -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CACHE_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .context("cannot determine a user directory; set VICTORY_CACHE_DIR and VICTORY_DATA_DIR")?;
    Ok(base.join("victory").join(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn environment_overrides_file() {
        let file = FilePaths {
            game_dir: Some("from-file".into()),
            cache_dir: Some("cache-from-file".into()),
            ..Default::default()
        };
        let env = |k: &str| (k == "VICTORY_GAME_DIR").then(|| OsString::from("from-env"));
        let cfg = Config::resolve(
            FileConfig {
                paths: file,
                ..Default::default()
            },
            None,
            env,
        )
        .unwrap();
        assert_eq!(cfg.game_dir.unwrap(), PathBuf::from("from-env"));
        assert_eq!(cfg.cache_dir, PathBuf::from("cache-from-file"));
    }

    #[test]
    fn empty_env_values_are_ignored() {
        let file = FilePaths {
            game_dir: Some("from-file".into()),
            ..Default::default()
        };
        let env = |k: &str| (k == "VICTORY_GAME_DIR").then(OsString::new);
        let cfg = Config::resolve(
            FileConfig {
                paths: file,
                ..Default::default()
            },
            None,
            env,
        )
        .unwrap();
        assert_eq!(cfg.game_dir.unwrap(), PathBuf::from("from-file"));
    }

    #[test]
    fn empty_test_titles_env_is_ignored() {
        let file = FilePaths {
            test_titles: vec!["from-file".into()],
            ..Default::default()
        };
        let env = |k: &str| (k == "VICTORY_TEST_TITLES").then(OsString::new);
        let cfg = Config::resolve(
            FileConfig {
                paths: file,
                ..Default::default()
            },
            None,
            env,
        )
        .unwrap();
        assert_eq!(cfg.test_titles, vec![PathBuf::from("from-file")]);
    }

    #[test]
    fn test_titles_env_splits_paths() {
        let value = std::env::join_paths(["first", "second"]).unwrap();
        let file = FilePaths::default();
        let env = |k: &str| (k == "VICTORY_TEST_TITLES").then(|| value.clone());
        let cfg = Config::resolve(
            FileConfig {
                paths: file,
                ..Default::default()
            },
            None,
            env,
        )
        .unwrap();
        assert_eq!(
            cfg.test_titles,
            vec![PathBuf::from("first"), PathBuf::from("second")]
        );
    }

    #[test]
    fn empty_config_env_is_ignored() {
        let empty = |k: &str| (k == "VICTORY_CONFIG").then(OsString::new);
        assert_eq!(explicit_config_path(empty), None);
        let set = |k: &str| (k == "VICTORY_CONFIG").then(|| OsString::from("custom.toml"));
        assert_eq!(
            explicit_config_path(set),
            Some(PathBuf::from("custom.toml"))
        );
    }

    #[test]
    fn example_config_parses() {
        let text = include_str!("../../../victory.example.toml");
        let parsed: FileConfig = toml::from_str(text).unwrap();
        assert!(parsed.paths.game_dir.is_some());
    }

    #[test]
    fn configured_memory_survives_resolution() {
        let parsed: FileConfig = toml::from_str("[memory]\nlow_ram = true\narena_mb = 32").unwrap();
        let cfg = Config::resolve(parsed, None, |_| None).unwrap();
        assert_eq!(cfg.memory.low_ram, Some(true));
        assert_eq!(cfg.memory.arena_mb, Some(32));
    }

    #[test]
    fn memory_environment_overrides_only_named_values() {
        let cfg: FileConfig =
            toml::from_str("[memory]\nlow_ram = false\narena_mb = 24\nstaging_kept = 1").unwrap();
        let resolved = Config::resolve(cfg, None, |key| match key {
            "VICTORY_LOW_RAM" => Some("1".into()),
            "VICTORY_ARENA_MB" => Some("48".into()),
            _ => None,
        })
        .unwrap();
        assert_eq!(resolved.memory.low_ram, Some(true));
        assert_eq!(resolved.memory.arena_mb, Some(48));
        assert_eq!(resolved.memory.staging_kept, Some(1));
    }

    #[test]
    fn cache_inside_repo_is_refused() {
        let inside = Path::new(env!("CARGO_MANIFEST_DIR")).join("cache");
        let cfg = Config {
            cache_dir: inside,
            ..Default::default()
        };
        assert!(cfg.cache_subdir("x").is_err());
    }

    #[test]
    fn data_inside_repo_is_refused() {
        let inside = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        let cfg = Config {
            data_dir: inside,
            ..Default::default()
        };
        assert!(cfg.data_subdir("x").is_err());
    }
}
