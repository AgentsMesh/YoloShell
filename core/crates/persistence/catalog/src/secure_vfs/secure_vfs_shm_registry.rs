use super::secure_vfs_identity::FileIdentity;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

const REFERENCE_OVERFLOW: &str = "SQLite SHM reference count exhausted";
const RELEASE_WITHOUT_REFERENCE: &str = "SQLite SHM release has no retained reference";

#[derive(Default)]
struct SharedShm {
    delete_requested: bool,
    references: usize,
}

static REGISTRY: OnceLock<Mutex<HashMap<FileIdentity, SharedShm>>> = OnceLock::new();

pub(super) fn retain(identity: FileIdentity) -> Result<()> {
    let mut registry = lock_registry();
    let shared = registry.entry(identity).or_default();
    shared.references = shared
        .references
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!(REFERENCE_OVERFLOW))?;
    Ok(())
}

pub(super) fn release(identity: FileIdentity, delete_requested: bool) -> Result<bool> {
    let mut registry = lock_registry();
    let shared = registry
        .get_mut(&identity)
        .ok_or_else(|| anyhow::anyhow!(RELEASE_WITHOUT_REFERENCE))?;
    anyhow::ensure!(shared.references != 0, RELEASE_WITHOUT_REFERENCE);
    shared.delete_requested |= delete_requested;
    shared.references -= 1;
    if shared.references != 0 {
        return Ok(false);
    }
    let delete = shared.delete_requested;
    registry.remove(&identity);
    Ok(delete)
}

fn lock_registry() -> MutexGuard<'static, HashMap<FileIdentity, SharedShm>> {
    REGISTRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
