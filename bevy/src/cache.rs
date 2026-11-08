use crate::built::Built;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

const FORMAT: u32 = 1;
const KEPT: usize = 16;

type Slot = Arc<OnceLock<Result<Arc<Built>, String>>>;

#[derive(Default)]
pub struct Cache {
    dir: Option<PathBuf>,
    slots: Mutex<Vec<(u64, Slot)>>,
    builds: std::sync::atomic::AtomicUsize,
}

impl Cache {
    pub fn new(dir: Option<PathBuf>) -> Cache {
        Cache {
            dir,
            ..Default::default()
        }
    }

    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    pub fn builds(&self) -> usize {
        self.builds.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn get_or_build(
        &self,
        key: u64,
        build: impl FnOnce() -> Result<Built, String>,
    ) -> Result<Arc<Built>, String> {
        let slot = {
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
        };
        slot.get_or_init(|| {
            if let Some(built) = self.read(key) {
                return Ok(Arc::new(built));
            }
            let built = build()?;
            self.builds.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.write(key, &built);
            Ok(Arc::new(built))
        })
        .clone()
    }

    fn file(&self, key: u64) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(format!("{key:016x}.bin")))
    }

    fn read(&self, key: u64) -> Option<Built> {
        let bytes = std::fs::read(self.file(key)?).ok()?;
        let (format, body) = bytes.split_at_checked(4)?;
        (u32::from_le_bytes(format.try_into().ok()?) == FORMAT)
            .then(|| bitcode::decode(body).ok())
            .flatten()
    }

    fn write(&self, key: u64, built: &Built) {
        let (Some(dir), Some(file)) = (self.dir.as_ref(), self.file(key)) else {
            return;
        };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let mut bytes = FORMAT.to_le_bytes().to_vec();
        bytes.extend(bitcode::encode(built));
        let partial = file.with_extension(format!("{}.part", std::process::id()));
        if std::fs::write(&partial, bytes).is_ok() && std::fs::rename(&partial, &file).is_err() {
            let _ = std::fs::remove_file(partial);
        }
    }
}

pub fn default_dir() -> Option<PathBuf> {
    if cfg!(any(target_arch = "wasm32", target_os = "android", target_os = "ios")) {
        return None;
    }
    if std::env::var_os("GCAD_NO_PART_CACHE").is_some() {
        return None;
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(PathBuf::from))?;
    Some(base.join("gcad").join("bevy"))
}

pub fn fold(key: u64, parts: &[&[u8]]) -> u64 {
    let mut hash = key ^ 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for b in (part.len() as u64).to_le_bytes().iter().chain(part.iter()) {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}
