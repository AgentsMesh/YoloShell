use super::constants::UNKNOWN_BOOT_ID_PREFIX;
#[cfg(not(target_os = "macos"))]
use super::procfs;
use std::sync::{Mutex, OnceLock};

#[path = "boot_identity_digest.rs"]
mod digest;
pub use digest::BootIdentity;
#[cfg(target_os = "macos")]
#[path = "boot_identity_macos.rs"]
mod macos;

static BOOT_ID: BootIdCache = BootIdCache::new();
#[cfg(test)]
static RESOLUTION_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

#[cfg(target_os = "macos")]
pub const MAX_BOOT_IDENTITY_BYTES: usize =
    macos::MACOS_BOOT_PREFIX.len() + uuid::fmt::Hyphenated::LENGTH;
#[cfg(not(target_os = "macos"))]
pub const MAX_BOOT_IDENTITY_BYTES: usize = uuid::fmt::Hyphenated::LENGTH;

struct BootIdCache {
    known: OnceLock<String>,
    resolution: Mutex<()>,
}

impl BootIdCache {
    const fn new() -> Self {
        Self {
            known: OnceLock::new(),
            resolution: Mutex::new(()),
        }
    }

    fn resolve(
        &self,
        resolver: impl FnOnce() -> Option<String>,
        fallback: impl FnOnce() -> String,
    ) -> String {
        if let Some(known) = self.known.get() {
            return known.clone();
        }
        let _resolution = self
            .resolution
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(known) = self.known.get() {
            return known.clone();
        }
        if let Some(known) = resolver()
            .filter(|value| boot_identity_relation(value, value) == BootIdentityRelation::Same)
        {
            let _ = self.known.set(known);
            if let Some(known) = self.known.get() {
                return known.clone();
            }
        }
        fallback()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BootIdentityRelation {
    Same,
    Different,
    Unknown,
}

pub fn boot_id() -> String {
    BOOT_ID.resolve(resolve_known_boot_id, unknown_boot_id)
}

pub fn boot_identity_relation(left: &str, right: &str) -> BootIdentityRelation {
    if left.is_empty()
        || right.is_empty()
        || left.starts_with(UNKNOWN_BOOT_ID_PREFIX)
        || right.starts_with(UNKNOWN_BOOT_ID_PREFIX)
    {
        return BootIdentityRelation::Unknown;
    }
    #[cfg(target_os = "macos")]
    if left.starts_with(macos::MACOS_BOOT_PREFIX) || right.starts_with(macos::MACOS_BOOT_PREFIX) {
        return macos::relation(left, right);
    }
    if left == right {
        BootIdentityRelation::Same
    } else {
        BootIdentityRelation::Different
    }
}

#[cfg(test)]
pub(crate) fn resolution_count() -> usize {
    RESOLUTION_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

fn resolve_known_boot_id() -> Option<String> {
    #[cfg(test)]
    RESOLUTION_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    #[cfg(target_os = "linux")]
    if let Ok(value) = std::fs::read_to_string(procfs::boot_id_path()) {
        let value = value.trim();
        if !value.is_empty() {
            return Some(value.to_owned());
        }
    }
    #[cfg(target_os = "macos")]
    if let Some(value) = macos::resolve() {
        return Some(value);
    }
    None
}

fn unknown_boot_id() -> String {
    format!("{UNKNOWN_BOOT_ID_PREFIX}{}", std::process::id())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/boot_identity_tests.rs"]
mod tests;
