use super::retained_artifact::{RetainedArtifact, RetainedEntry};
use super::retained_namespace::NamespaceInner;
use super::retained_registry_identity::{
    candidate_entries, ensure_candidate_disjoint, ensure_entries_disjoint, same_entry_slot,
    same_slot,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, Weak};

#[cfg(test)]
use std::cell::RefCell;

const MAIN_ARTIFACT_MISSING: &str = "retained catalog has no main artifact";

#[derive(Default)]
struct Registry {
    artifacts: HashMap<PathBuf, Weak<RetainedEntry>>,
    namespaces: Vec<Weak<NamespaceInner>>,
}

static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();

pub(super) fn register(candidate: Arc<NamespaceInner>) -> Result<Arc<NamespaceInner>> {
    let mut registry = lock_registry();
    registry
        .namespaces
        .retain(|entry| entry.strong_count() != 0);
    let active = registry
        .namespaces
        .iter()
        .filter_map(Weak::upgrade)
        .collect::<Vec<_>>();
    let decision = (|| {
        for registered in &active {
            if same_slot(candidate.entries(), registered.entries())? {
                return Ok(Some(Arc::clone(registered)));
            }
            ensure_entries_disjoint(candidate.entries(), registered.entries())?;
        }
        Ok(None)
    })();
    if matches!(decision, Ok(None)) {
        for entry in candidate.entries() {
            registry
                .artifacts
                .insert(entry.path().to_path_buf(), entry.downgrade());
        }
        registry.namespaces.push(Arc::downgrade(&candidate));
    }
    drop(registry);
    drop(active);
    match decision {
        Ok(Some(registered)) => Ok(registered),
        Ok(None) => Ok(candidate),
        Err(error) => Err(error),
    }
}

pub(super) fn acquire(
    main: &crate::paths::private_file::PrivateFileEntry,
) -> Result<Option<Arc<NamespaceInner>>> {
    let candidates = candidate_entries(main)?;
    let mut registry = lock_registry();
    registry
        .namespaces
        .retain(|entry| entry.strong_count() != 0);
    let active = registry
        .namespaces
        .iter()
        .filter_map(Weak::upgrade)
        .collect::<Vec<_>>();
    run_acquire_test_hook();
    let decision = (|| {
        for registered in &active {
            let registered_main = registered
                .entries()
                .first()
                .context(MAIN_ARTIFACT_MISSING)?;
            if same_entry_slot(main, registered_main.entry())? {
                return Ok(Some(Arc::clone(registered)));
            }
            ensure_candidate_disjoint(&candidates, registered.entries())?;
        }
        Ok(None)
    })();
    drop(registry);
    drop(active);
    decision
}

pub(super) fn unregister(namespace: &NamespaceInner) {
    let mut registry = lock_registry();
    for entry in namespace.entries() {
        let remove = registry
            .artifacts
            .get(entry.path())
            .is_some_and(|registered| entry.ptr_eq(registered));
        if remove {
            registry.artifacts.remove(entry.path());
        }
    }
    registry.namespaces.retain(|registered| {
        registered.as_ptr() != std::ptr::from_ref(namespace) && registered.strong_count() != 0
    });
}

pub(super) fn resolve(path: &Path) -> Option<RetainedArtifact> {
    lock_registry()
        .artifacts
        .get(path)
        .and_then(RetainedArtifact::from_weak)
}

pub(super) fn is_registered(path: &Path) -> bool {
    resolve(path).is_some()
}

fn run_acquire_test_hook() {
    #[cfg(test)]
    run_acquire_locked();
}

#[cfg(test)]
type AcquireHook = Box<dyn FnOnce()>;

#[cfg(test)]
thread_local! {
    static ACQUIRE_LOCKED_HOOK: RefCell<Option<AcquireHook>> = RefCell::new(None);
}

#[cfg(test)]
pub(super) fn set_acquire_locked(hook: impl FnOnce() + 'static) {
    ACQUIRE_LOCKED_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_acquire_locked() {
    let hook = ACQUIRE_LOCKED_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

fn lock_registry() -> MutexGuard<'static, Registry> {
    REGISTRY
        .get_or_init(|| Mutex::new(Registry::default()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
