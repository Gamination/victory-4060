//! `victory precompile`: everything a run would translate and make on its
//! way, translated and made first, so that a place seen for the first time
//! is not slow while its shaders and its pipelines are made: every shader
//! found in the title's files, and every one earlier runs recorded, into
//! the shader cache; then the host pipelines those runs drew with, into the
//! driver's own cache on disk (`warm`).
//!
//! What a translation is depends on the switches the run is given
//! (`shader_cache::build`), so this must be started with the same settings
//! as the run that follows it: the launchers do (`-Precompile`).
//!
//! A launch does this first (`scripts\play-nvidia.ps1`), and a walk of the
//! title is a minute's reading of its files: `--if-needed` skips it when the
//! last precompile saw the same build, switches, files and cache (`state`).
//!
//! The cache is on disk already; `--save` keeps it as one file and `--load`
//! puts it back (`bundle`), for a cache that was wiped.

mod bundle;
mod state;

use std::path::PathBuf;

use anyhow::Result;
use victory_hle::known_pipelines as known;

pub struct Options {
    /// The title's files (default: the configured game directory).
    pub paths: Vec<PathBuf>,
    /// Shader threads (default: every logical core); pipeline processes
    /// (default: two thirds of them).
    pub jobs: Option<usize>,
    /// Translate the shaders only: make no host pipelines.
    pub shaders_only: bool,
    /// Every pipeline earlier runs created, not only those they drew with.
    pub all: bool,
    /// At most this many pipelines.
    pub limit: Option<usize>,
    /// Those made before too.
    pub again: bool,
    /// Write the cache into this file when done (default:
    /// `precompiled.vpc`, beside the cache folder).
    pub save: Option<Option<PathBuf>>,
    /// Put the entries of this file into the cache before translating
    /// (default: `precompiled.vpc`, beside the cache folder).
    pub load: Option<Option<PathBuf>>,
    /// Skip the walk when the last precompile saw the same build, switches,
    /// title files and cache (what a launch does).
    pub if_needed: bool,
}

/// Where `--save` and `--load` look when given no file: beside the cache
/// folder, so that wiping the cache does not wipe what was saved from it (a
/// cache configured with no parent has the file inside it, as there is no
/// better place).
fn default_file(cfg: &victory_core::Config) -> PathBuf {
    match cfg.cache_dir.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.join("precompiled.vpc"),
        _ => cfg.cache_dir.join("precompiled.vpc"),
    }
}

pub fn run(options: Options) -> Result<()> {
    let cfg = victory_core::Config::load()?;
    let shaders = cfg.cache_subdir("shaders")?;
    victory_hle::modules::d3d12x::use_shader_cache(shaders.clone());
    let dir = shaders.with_file_name("pipelines");
    let default = || default_file(&cfg);
    let began = std::time::Instant::now();
    if let Some(file) = &options.load {
        // A file the launcher asks for is only there after a save; one named
        // on the command line has to be.
        let required = matches!(options.load, Some(Some(_)));
        let file = file.clone().unwrap_or_else(default);
        match bundle::load(&shaders, &file, required)? {
            Some(loaded) => loaded.report(&file),
            None => println!("loaded       nothing: {} is not there yet", file.display()),
        }
    }
    let paths = crate::default_paths(options.paths)?;
    let seen = cfg.cache_dir.join("precompiled.state");
    let now = options.if_needed.then(|| state::of(&shaders, &paths));
    // The pipelines are warmed either way (it skips those it has made); only
    // the walk, which reads every file of the title, is worth skipping.
    let skip = match &now {
        Some(now) => !options.all && !options.again && state::unchanged(&seen, now),
        None => false,
    };
    if skip {
        println!("shaders      nothing new since the last one: the cache has them all");
    } else {
        let done = crate::translate::precompile(paths, options.jobs)?;
        done.report();
        if let Some(now) = &now {
            state::record(&seen, now)?;
        }
    }
    if !options.shaders_only {
        // Runs with `VICTORY_PIPELINE_BOOK=1` wrote the pipelines down;
        // `warm` makes them in processes of their own, so that a compiler
        // that crashes or hangs costs one pipeline.
        let counted = known::count(&dir);
        if counted == 0 {
            println!(
                "pipelines    none recorded yet: play once with VICTORY_PIPELINE_BOOK=1 \
                 (scripts\\play-nvidia.ps1 and scripts\\play.ps1 set it), then precompile again"
            );
        } else {
            crate::warm::run(
                options.jobs,
                options.limit,
                options.again,
                !options.all,
                None,
            )?;
        }
    }
    if let Some(file) = &options.save {
        let file = file.clone().unwrap_or_else(default);
        bundle::save(&shaders, &file)?.report(&file);
    }
    println!(
        "precompiled  in {:.0} s; the cache is {}",
        began.elapsed().as_secs_f64(),
        cfg.cache_dir.display()
    );
    Ok(())
}
