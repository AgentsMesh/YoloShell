use super::secure_vfs_alias::SecureAlias;
use super::secure_vfs_file::{ArtifactKind, SecureDescriptor};
use anyhow::Context;
use rusqlite::ffi;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

struct FileState {
    alias: SecureAlias,
    artifact_path: PathBuf,
    descriptor: SecureDescriptor,
    shm_descriptor: Option<SecureDescriptor>,
    original_methods: usize,
    _wrapper_methods: Box<ffi::sqlite3_io_methods>,
}

static FILE_STATES: OnceLock<Mutex<HashMap<usize, FileState>>> = OnceLock::new();

include!("secure_vfs_state_install.rs");
include!("secure_vfs_state_close.rs");

pub(super) fn state_snapshot(file: *mut ffi::sqlite3_file) -> Option<(PathBuf, PathBuf, usize)> {
    lock_states().get(&(file as usize)).map(|state| {
        (
            state.alias.shm_path(),
            state.artifact_path.clone(),
            state.original_methods,
        )
    })
}

pub(super) fn verify_shm_descriptor(
    file: *mut ffi::sqlite3_file,
    descriptor: &SecureDescriptor,
) -> anyhow::Result<()> {
    let states = lock_states();
    let state = states
        .get(&(file as usize))
        .context("SQLite file state disappeared")?;
    if let Some(expected) = &state.shm_descriptor {
        anyhow::ensure!(
            expected.same_object(descriptor),
            "SQLite SHM identity changed while mapped"
        );
    }
    Ok(())
}

pub(super) fn retain_shm_descriptor(
    file: *mut ffi::sqlite3_file,
    descriptor: SecureDescriptor,
) -> anyhow::Result<bool> {
    let mut states = lock_states();
    let state = states
        .get_mut(&(file as usize))
        .context("SQLite file state disappeared")?;
    if let Some(expected) = &state.shm_descriptor {
        anyhow::ensure!(
            expected.same_object(&descriptor),
            "SQLite SHM identity changed while retaining descriptor"
        );
    } else {
        state.shm_descriptor = Some(descriptor);
        return Ok(true);
    }
    Ok(false)
}

pub(super) fn take_shm_descriptor(
    file: *mut ffi::sqlite3_file,
) -> anyhow::Result<Option<SecureDescriptor>> {
    lock_states()
        .get_mut(&(file as usize))
        .context("SQLite file state disappeared")
        .map(|state| state.shm_descriptor.take())
}

pub(super) fn create_shm_link(
    file: *mut ffi::sqlite3_file,
    descriptor: &SecureDescriptor,
) -> anyhow::Result<()> {
    let states = lock_states();
    let state = states
        .get(&(file as usize))
        .context("SQLite file state disappeared")?;
    state.alias.create_shm_link(descriptor)
}

pub(super) fn remove_shm_link(
    file: *mut ffi::sqlite3_file,
    descriptor: &SecureDescriptor,
) -> anyhow::Result<()> {
    let states = lock_states();
    let state = states
        .get(&(file as usize))
        .context("SQLite file state disappeared")?;
    state.alias.remove_shm_link(descriptor)
}

pub(super) fn original_methods(file: *mut ffi::sqlite3_file) -> Option<usize> {
    lock_states()
        .get(&(file as usize))
        .map(|state| state.original_methods)
}

pub(super) fn verify_retained(file: *mut ffi::sqlite3_file) -> anyhow::Result<()> {
    let states = lock_states();
    let state = states
        .get(&(file as usize))
        .context("SQLite retained file state disappeared")?;
    anyhow::ensure!(
        state.descriptor.is_retained(),
        "SQLite file is not retained"
    );
    state.descriptor.verify_current()
}

pub(super) fn set_alias_sealed(file: *mut ffi::sqlite3_file, sealed: bool) -> anyhow::Result<()> {
    let states = lock_states();
    let state = states
        .get(&(file as usize))
        .context("SQLite file state disappeared")?;
    if sealed {
        state.alias.seal()
    } else {
        state.alias.unseal()
    }
}

fn lock_states() -> MutexGuard<'static, HashMap<usize, FileState>> {
    states()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn states() -> &'static Mutex<HashMap<usize, FileState>> {
    FILE_STATES.get_or_init(|| Mutex::new(HashMap::new()))
}
