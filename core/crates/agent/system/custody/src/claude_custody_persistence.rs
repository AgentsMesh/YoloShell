use super::types::*;
use anyhow::{Result, ensure};
use std::path::Path;

pub(super) fn update<T>(
    path: &Path,
    mutate: impl FnOnce(&mut CustodyLedger) -> Result<T>,
) -> Result<T> {
    match super::context::ManagedCustodySession::capture(path) {
        Ok(session) => update_managed(&session, mutate),
        Err(error) => {
            #[cfg(test)]
            if !super::context::ledger_has_managed_shape(path) {
                return update_unmanaged_for_test(path, mutate);
            }
            Err(error)
        }
    }
}

fn update_managed<T>(
    session: &super::context::ManagedCustodySession,
    mutate: impl FnOnce(&mut CustodyLedger) -> Result<T>,
) -> Result<T> {
    let guard = session.guard()?;
    session.require_current_manifest(&guard)?;
    guard.require_no_pending_intent()?;
    let (mut ledger, expected) = match guard.read_optional_with_identity(session.ledger_path())? {
        Some((ledger, identity)) => {
            ensure!(identity.length() <= MAX_LEDGER_BYTES, "{LEDGER_TOO_LARGE}");
            (ledger, Some(identity))
        }
        None => (empty(), None),
    };
    validate(&ledger)?;
    let result = mutate(&mut ledger)?;
    validate_encoded(&ledger)?;
    run_before_write_hook()?;
    guard.require_no_pending_intent()?;
    guard.namespace().write_json_expected_with_commit_fence(
        session.ledger_path(),
        expected,
        &ledger,
        || {
            guard.require_no_pending_intent()?;
            session.require_current_manifest(&guard)
        },
    )?;
    run_after_commit_hook();
    Ok(result)
}

#[cfg(test)]
fn update_unmanaged_for_test<T>(
    path: &Path,
    mutate: impl FnOnce(&mut CustodyLedger) -> Result<T>,
) -> Result<T> {
    let entry = crate::paths::private_file::PrivateFileEntry::ensure(path)?;
    let _lock = entry.lock_parent_exclusive_wait(LOCK_TIMEOUT, LOCK_RETRY)?;
    let mut ledger = load(path)?;
    validate(&ledger)?;
    let result = mutate(&mut ledger)?;
    validate_encoded(&ledger)?;
    crate::paths::atomic_write_json(path, &ledger)?;
    Ok(result)
}

#[cfg(test)]
fn load(path: &Path) -> Result<CustodyLedger> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() == u64::default() => Ok(empty()),
        Ok(metadata) => {
            ensure!(metadata.len() <= MAX_LEDGER_BYTES, "{LEDGER_TOO_LARGE}");
            crate::paths::read_json(path)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(empty()),
        Err(error) => Err(error.into()),
    }
}

fn empty() -> CustodyLedger {
    CustodyLedger {
        schema_version: LEDGER_SCHEMA_VERSION,
        records: Vec::new(),
    }
}

pub(super) fn validate(ledger: &CustodyLedger) -> Result<()> {
    let mut invocations = std::collections::HashSet::new();
    ensure!(
        ledger.schema_version == LEDGER_SCHEMA_VERSION
            && ledger.records.len() <= MAX_RECORDS
            && ledger.records.iter().all(|record| {
                !record.spec.invocation_id.is_nil()
                    && invocations.insert(record.spec.invocation_id)
                    && valid_identity(&record.spec.target)
                    && valid_identity(&record.spec.worker)
                    && record.spec.guardians.iter().all(valid_identity)
                    && record.spec.guardians[PRIMARY_GUARDIAN_INDEX]
                        != record.spec.guardians[STANDBY_GUARDIAN_INDEX]
            }),
        "{LEDGER_INVALID}"
    );
    Ok(())
}

fn validate_encoded(ledger: &CustodyLedger) -> Result<()> {
    validate(ledger)?;
    let encoded = serde_json::to_vec(ledger)?;
    ensure!(
        encoded.len() as u64 <= MAX_LEDGER_BYTES,
        "{LEDGER_TOO_LARGE}"
    );
    Ok(())
}

fn valid_identity(identity: &DurableProcessIdentity) -> bool {
    !identity.boot_id.is_empty()
        && identity.pid != u32::default()
        && identity.start_time != u64::default()
}

const LEDGER_INVALID: &str = "Claude custody ledger is invalid";
pub(super) const LEDGER_TOO_LARGE: &str = "Claude custody ledger is too large";
const PRIMARY_GUARDIAN_INDEX: usize = 0;
const STANDBY_GUARDIAN_INDEX: usize = 1;

#[cfg(test)]
type BeforeWriteHook = Option<Box<dyn FnOnce() -> Result<()>>>;

#[cfg(test)]
thread_local! {
    static BEFORE_WRITE_HOOK: std::cell::RefCell<BeforeWriteHook> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_before_write_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    BEFORE_WRITE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_before_write_hook() -> Result<()> {
    BEFORE_WRITE_HOOK.with(|slot| match slot.borrow_mut().take() {
        Some(hook) => hook(),
        None => Ok(()),
    })
}

#[cfg(not(test))]
fn run_before_write_hook() -> Result<()> {
    Ok(())
}

#[cfg(test)]
type AfterCommitHook = Option<Box<dyn FnOnce()>>;

#[cfg(test)]
thread_local! {
    static AFTER_COMMIT_HOOK: std::cell::RefCell<AfterCommitHook> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_after_commit_hook(hook: impl FnOnce() + 'static) {
    AFTER_COMMIT_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_after_commit_hook() {
    AFTER_COMMIT_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(test))]
fn run_after_commit_hook() {}
