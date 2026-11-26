//! The shader cache as one file: `--save` writes every translation into it and
//! `--load` puts them back, so that a cache that was wiped, or a second machine
//! with the same build and settings, need not translate them again.
//!
//! The cache is on disk already (`<cache folder>/shaders`: two hex letters,
//! then the rest of the key); the file is how it travels, or how it survives a
//! wiped cache folder. Both hold the title's shaders, translated, and both stay
//! outside the repository (`docs/asset-policy.md`).

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail};

/// `VPCB`, then a version: a later format is not read as this one.
const MAGIC: &[u8; 4] = b"VPCB";
const VERSION: u32 = 1;
/// One entry is our encoding of a `HostShader`: kilobytes, not megabytes.
const MAX_ENTRY: u64 = 16 << 20;
/// A count no cache has: a damaged or foreign file.
const MAX_ENTRIES: u64 = 1 << 20;
/// The longest entry path the cache makes: two hex digits, `/`, 62 more.
const MAX_PATH: usize = 65;

const MIB: f64 = 1024.0 * 1024.0;

/// What `save` wrote.
#[derive(Debug)]
pub struct Saved {
    pub entries: usize,
    pub bytes: u64,
}

impl Saved {
    pub fn report(&self, file: &Path) {
        println!(
            "saved        {}, {:.1} MB, to {}",
            shaders(self.entries),
            self.bytes as f64 / MIB,
            file.display()
        );
    }
}

/// What `load` found, and what of it the cache did not have.
#[derive(Debug)]
pub struct Loaded {
    pub entries: usize,
    pub added: usize,
    pub bytes: u64,
}

impl Loaded {
    pub fn report(&self, file: &Path) {
        println!(
            "loaded       {}, {:.1} MB, from {} ({} in the cache already)",
            shaders(self.entries),
            self.bytes as f64 / MIB,
            file.display(),
            self.entries - self.added
        );
    }
}

/// "1 shader", "2 shaders".
fn shaders(n: usize) -> String {
    if n == 1 {
        "1 shader".to_owned()
    } else {
        format!("{n} shaders")
    }
}

/// Writes every entry of `cache` into `file`, one file.
pub fn save(cache: &Path, file: &Path) -> Result<Saved> {
    victory_core::ensure_outside_repo(file)?;
    let entries = entries(cache);
    if entries.is_empty() {
        bail!("no shader is in the cache yet: precompile first");
    }
    let (tmp, out) = staging(file)?;
    let mut out = BufWriter::new(out);
    let written = (|| -> Result<Saved> {
        out.write_all(MAGIC)?;
        out.write_all(&VERSION.to_le_bytes())?;
        out.write_all(&(entries.len() as u64).to_le_bytes())?;
        let mut bytes = 0;
        for (rel, path) in &entries {
            let data =
                std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
            out.write_all(&(rel.len() as u32).to_le_bytes())?;
            out.write_all(rel.as_bytes())?;
            out.write_all(&(data.len() as u64).to_le_bytes())?;
            out.write_all(&data)?;
            out.write_all(blake3::hash(&data).as_bytes())?;
            bytes += data.len() as u64;
        }
        out.flush()?;
        out.get_ref().sync_all()?;
        Ok(Saved {
            entries: entries.len(),
            bytes,
        })
    })();
    match written {
        Ok(saved) => {
            drop(out);
            std::fs::rename(&tmp, file).with_context(|| format!("writing {}", file.display()))?;
            Ok(saved)
        }
        Err(why) => {
            drop(out);
            let _ = std::fs::remove_file(&tmp);
            Err(why)
        }
    }
}

/// Puts the entries of `file` into `cache`, those it does not have yet.
///
/// With `required` off, a file that is not there is nothing to load, not an
/// error: every launch loads what a precompile saved, and the first one has
/// saved nothing yet.
pub fn load(cache: &Path, file: &Path, required: bool) -> Result<Option<Loaded>> {
    if !required && !file.is_file() {
        return Ok(None);
    }
    victory_core::ensure_outside_repo(file)?;
    let source = File::open(file).with_context(|| format!("opening {}", file.display()))?;
    let mut source = BufReader::new(source);
    let mut magic = [0u8; 4];
    fill(&mut source, &mut magic, file)?;
    if &magic != MAGIC {
        bail!("{} is not a saved shader cache", file.display());
    }
    let version = u32_at(&mut source, file)?;
    if version != VERSION {
        bail!(
            "{} is a format {version} file (this build writes {VERSION})",
            file.display()
        );
    }
    let count = u64_at(&mut source, file)?;
    if count > MAX_ENTRIES {
        bail!(
            "{} names {count} entries: not a saved cache",
            file.display()
        );
    }
    let mut added = 0;
    let mut bytes = 0;
    for n in 0..count {
        let len = u32_at(&mut source, file)? as usize;
        if len == 0 || len > MAX_PATH {
            bail!("{}: entry {n} has no path", file.display());
        }
        let mut path = vec![0u8; len];
        fill(&mut source, &mut path, file)?;
        let rel = String::from_utf8(path)
            .with_context(|| format!("{}: entry {n} has no path", file.display()))?;
        if !is_entry(&rel) {
            bail!("{}: entry {n} ({rel}) is not a cache entry", file.display());
        }
        let size = u64_at(&mut source, file)?;
        if size > MAX_ENTRY {
            bail!("{}: entry {n} is {size} bytes", file.display());
        }
        let mut data = vec![0u8; size as usize];
        fill(&mut source, &mut data, file)?;
        let mut hash = [0u8; 32];
        fill(&mut source, &mut hash, file)?;
        if hash != *blake3::hash(&data).as_bytes() {
            bail!("{}: entry {n} ({rel}) is damaged", file.display());
        }
        let target = cache.join(&rel);
        if target.is_file() {
            continue;
        }
        put(&target, &data)?;
        added += 1;
        bytes += size;
    }
    Ok(Some(Loaded {
        entries: count as usize,
        added,
        bytes,
    }))
}

/// The cache's entries: their path relative to it, and where they are.
fn entries(cache: &Path) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    for entry in walkdir::WalkDir::new(cache)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let Some(rel) = relative(cache, &path) else {
            continue;
        };
        if is_entry(&rel) {
            found.push((rel, path));
        }
    }
    found.sort();
    found
}

/// The path of `path` below `root`, with `/` between the parts.
fn relative(root: &Path, path: &Path) -> Option<String> {
    let mut rel = String::new();
    for part in path.strip_prefix(root).ok()?.components() {
        let part = part.as_os_str().to_str()?;
        if !rel.is_empty() {
            rel.push('/');
        }
        rel.push_str(part);
    }
    Some(rel)
}

/// Whether `rel` is one entry of the cache: two hex letters, `/`, 62 more.
fn is_entry(rel: &str) -> bool {
    let hex = |s: &str| {
        s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    let Some((first, rest)) = rel.split_once('/') else {
        return false;
    };
    first.len() == 2 && rest.len() == 62 && !rest.contains('/') && hex(first) && hex(rest)
}

/// A temporary file next to `file`, and the writer for it.
fn staging(file: &Path) -> Result<(PathBuf, File)> {
    let dir = match file.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let name = file
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("precompiled.vpc"));
    let tmp = dir.join(format!(
        ".{}.tmp-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    let out = File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
    Ok((tmp, out))
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write-then-rename, as the cache does, so a reader never sees half an entry.
fn put(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().context("a cache entry has a parent")?;
    std::fs::create_dir_all(dir)?;
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(".tmp-{}-{n}", std::process::id()));
    let written = std::fs::write(&tmp, data);
    let published = written.and_then(|()| std::fs::rename(&tmp, path));
    if published.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    published.with_context(|| format!("writing {}", path.display()))
}

fn fill(source: &mut impl Read, buf: &mut [u8], file: &Path) -> Result<()> {
    source
        .read_exact(buf)
        .with_context(|| format!("{} is cut short", file.display()))
}

fn u32_at(source: &mut impl Read, file: &Path) -> Result<u32> {
    let mut buf = [0u8; 4];
    fill(source, &mut buf, file)?;
    Ok(u32::from_le_bytes(buf))
}

fn u64_at(source: &mut impl Read, file: &Path) -> Result<u64> {
    let mut buf = [0u8; 8];
    fill(source, &mut buf, file)?;
    Ok(u64::from_le_bytes(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("victory-bundle-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    /// One entry's name, in the shape the cache makes them.
    fn name(seed: u8) -> String {
        let rest: String = (0..62)
            .map(|i| char::from_digit((seed as u32 + i) % 16, 16).unwrap())
            .collect();
        format!("{seed:02x}/{rest}")
    }

    fn store(cache: &Path, rel: &str, data: &[u8]) {
        let path = cache.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, data).unwrap();
    }

    #[test]
    fn saved_shaders_come_back() {
        let root = dir("round-trip");
        let cache = root.join("shaders");
        let file = root.join("precompiled.vpc");
        let (a, b) = (name(3), name(5));
        store(&cache, &a, b"one translation");
        store(&cache, &b, b"another one");
        // (An interrupted write is not an entry.)
        std::fs::write(cache.join(".tmp-1-0"), b"half").unwrap();

        let saved = save(&cache, &file).unwrap();
        assert_eq!(saved.entries, 2);
        assert_eq!(saved.bytes, 15 + 11);

        std::fs::remove_dir_all(&cache).unwrap();
        let loaded = load(&cache, &file, true).unwrap().unwrap();
        assert_eq!((loaded.entries, loaded.added, loaded.bytes), (2, 2, 26));
        assert_eq!(std::fs::read(cache.join(&a)).unwrap(), b"one translation");
        assert_eq!(std::fs::read(cache.join(&b)).unwrap(), b"another one");

        // What the cache has already is not written again.
        let again = load(&cache, &file, true).unwrap().unwrap();
        assert_eq!((again.entries, again.added), (2, 0));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_damaged_file_puts_nothing_back() {
        let root = dir("damaged");
        let cache = root.join("shaders");
        let file = root.join("precompiled.vpc");
        let a = name(9);
        store(&cache, &a, b"a translation");
        save(&cache, &file).unwrap();
        let mut bytes = std::fs::read(&file).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        std::fs::write(&file, &bytes).unwrap();
        std::fs::remove_dir_all(&cache).unwrap();

        let why = load(&cache, &file, true).unwrap_err().to_string();
        assert!(why.contains("damaged"), "{why}");
        assert!(!cache.join(&a).exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn what_is_not_ours_is_not_read() {
        let root = dir("foreign");
        let cache = root.join("shaders");
        std::fs::create_dir_all(&cache).unwrap();
        let file = root.join("other.vpc");
        std::fs::write(&file, b"JUNK, not a cache").unwrap();
        assert!(load(&cache, &file, true).is_err());
        // (A header that stops in the middle is not read either.)
        std::fs::write(&file, b"VPCB\x01\x00").unwrap();
        assert!(load(&cache, &file, true).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_entry_outside_the_cache_is_refused() {
        let root = dir("outside");
        let cache = root.join("shaders");
        std::fs::create_dir_all(&cache).unwrap();
        let file = root.join("escape.vpc");
        let rel = "../outside";
        let mut bytes = Vec::new();
        bytes.extend(MAGIC);
        bytes.extend(VERSION.to_le_bytes());
        bytes.extend(1u64.to_le_bytes());
        bytes.extend((rel.len() as u32).to_le_bytes());
        bytes.extend(rel.as_bytes());
        bytes.extend(5u64.to_le_bytes());
        bytes.extend(blake3::hash(b"hello").as_bytes());
        bytes.extend(b"hello");
        std::fs::write(&file, &bytes).unwrap();

        let why = load(&cache, &file, true).unwrap_err().to_string();
        assert!(why.contains("not a cache entry"), "{why}");
        assert!(!root.join("outside").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_that_is_not_there_is_nothing_to_load() {
        let root = dir("absent");
        let cache = root.join("shaders");
        std::fs::create_dir_all(&cache).unwrap();
        let file = root.join("precompiled.vpc");
        // (What a launcher asks for: nothing saved yet is not a failure.)
        assert!(load(&cache, &file, false).unwrap().is_none());
        // (What was named on the command line has to be there.)
        assert!(load(&cache, &file, true).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_cache_has_nothing_to_save() {
        let root = dir("empty");
        assert!(save(&root.join("shaders"), &root.join("precompiled.vpc")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
