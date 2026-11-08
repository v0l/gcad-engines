use crate::built::Built;
use crate::source::{self, AssetCache, Pruned};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

pub const FORMAT: u32 = 1;
pub const EXTENSION: &str = "bin";
pub const BAKED_DIR: &str = "gcad-baked";
pub const DEFAULT_MAX_BYTES: u64 = source::DEFAULT_MAX_BYTES;
pub const DEFAULT_MAX_AGE: Duration = source::DEFAULT_MAX_AGE;
const KEPT: usize = 16;

type Slot = Arc<OnceLock<Result<Arc<Built>, String>>>;

#[derive(Default)]
pub struct Cache {
    disk: Option<AssetCache>,
    slots: Mutex<Vec<(u64, Slot)>>,
    used: Mutex<HashSet<u64>>,
    pub(crate) builds: AtomicUsize,
    pub(crate) hits_baked: AtomicUsize,
    pub(crate) hits_disk: AtomicUsize,
    pub(crate) hits_memory: AtomicUsize,
}

pub fn format_tag() -> String {
    format!("gcad-bevy-bitcode-{FORMAT}")
}

pub fn encode(built: &Built) -> Vec<u8> {
    let mut bytes = FORMAT.to_le_bytes().to_vec();
    bytes.extend(bitcode::encode(built));
    bytes
}

pub fn decode(bytes: &[u8]) -> Option<Built> {
    let (format, body) = bytes.split_at_checked(4)?;
    (u32::from_le_bytes(format.try_into().ok()?) == FORMAT)
        .then(|| bitcode::decode(body).ok())
        .flatten()
}

pub fn file_name(key: u64) -> String {
    source::file_name(key, EXTENSION)
}

impl Cache {
    pub fn new(dir: Option<PathBuf>) -> Cache {
        Cache {
            disk: dir.map(|d| AssetCache::with_extension(d, EXTENSION)),
            ..Default::default()
        }
    }

    pub fn dir(&self) -> Option<&Path> {
        self.disk.as_ref().map(AssetCache::dir)
    }

    pub fn count(counter: &AtomicUsize) -> usize {
        counter.load(Ordering::Relaxed)
    }

    pub fn bytes_on_disk(&self) -> u64 {
        self.disk.as_ref().map_or(0, AssetCache::bytes)
    }

    pub fn keep(&self, keys: &HashSet<u64>) {
        self.used.lock().unwrap_or_else(|e| e.into_inner()).extend(keys);
    }

    pub fn prune(&self, max_bytes: u64, max_age: Duration) -> Pruned {
        let keep = self.used.lock().unwrap_or_else(|e| e.into_inner()).clone();
        self.disk
            .as_ref()
            .map_or_else(Pruned::default, |d| d.prune(max_bytes, max_age, &keep))
    }

    fn slot(&self, key: u64) -> Slot {
        let mut slots = self.slots.lock().unwrap_or_else(|e| e.into_inner());
        slots.retain(|(_, s)| Arc::strong_count(s) > 1 || s.get().is_some_and(Result::is_ok));
        match slots.iter().find(|(k, _)| *k == key) {
            Some((_, s)) => s.clone(),
            None => {
                let s = Slot::default();
                slots.push((key, s.clone()));
                if slots.len() > KEPT {
                    slots.remove(0);
                }
                s
            }
        }
    }

    pub fn in_memory(&self, key: u64) -> bool {
        let slots = self.slots.lock().unwrap_or_else(|e| e.into_inner());
        slots.iter().any(|(k, s)| *k == key && s.get().is_some_and(Result::is_ok))
    }

    pub fn build_uncached(&self, build: impl FnOnce() -> Result<Built, String>) -> Result<Arc<Built>, String> {
        let built = build()?;
        self.builds.fetch_add(1, Ordering::Relaxed);
        Ok(Arc::new(built))
    }

    pub fn get_or_build(
        &self,
        key: u64,
        baked: Option<Vec<u8>>,
        meta: String,
        build: impl FnOnce() -> Result<Built, String>,
    ) -> Result<Arc<Built>, String> {
        self.used.lock().unwrap_or_else(|e| e.into_inner()).insert(key);
        let slot = self.slot(key);
        let mut ran = false;
        let result = slot
            .get_or_init(|| {
                ran = true;
                if let Some(built) = baked.as_deref().and_then(decode) {
                    self.hits_baked.fetch_add(1, Ordering::Relaxed);
                    return Ok(Arc::new(built));
                }
                if let Some(built) = self.read(key) {
                    self.hits_disk.fetch_add(1, Ordering::Relaxed);
                    return Ok(Arc::new(built));
                }
                let built = build()?;
                self.builds.fetch_add(1, Ordering::Relaxed);
                self.write(key, &built, &meta);
                Ok(Arc::new(built))
            })
            .clone();
        if !ran {
            self.hits_memory.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    fn read(&self, key: u64) -> Option<Built> {
        let disk = self.disk.as_ref()?;
        let built = decode(&disk.get(key, |b| b.len() > 4)?);
        if built.is_none() {
            disk.remove(key);
        }
        built
    }

    fn write(&self, key: u64, built: &Built, meta: &str) {
        if let Some(disk) = &self.disk {
            let _ = disk.put(key, &encode(built), meta);
        }
    }
}

#[cfg(target_os = "android")]
fn platform_dir() -> Option<PathBuf> {
    let files = bevy::android::ANDROID_APP.get()?.internal_data_path()?;
    Some(files.parent().map_or(files.clone(), |app| app.join("cache")))
}

#[cfg(target_os = "ios")]
fn platform_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("HOME")?).join("Library").join("Caches"))
}

#[cfg(target_arch = "wasm32")]
fn platform_dir() -> Option<PathBuf> {
    None
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
fn platform_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))
}

pub fn default_dir() -> Option<PathBuf> {
    if std::env::var_os("GCAD_NO_PART_CACHE").is_some() {
        return None;
    }
    Some(platform_dir()?.join("gcad").join("bevy"))
}
