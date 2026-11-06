#[cfg(any(test, feature = "adversarial-test-hooks"))]
use anyhow::Context;
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "claude_custody_context.rs"]
mod context;
#[path = "claude_custody_persistence.rs"]
mod persistence;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "claude_custody_reboot.rs"]
mod reboot;
#[path = "claude_custody_reboot_validation.rs"]
mod reboot_validation;
#[path = "claude_custody_state.rs"]
mod state;
#[path = "claude_custody_types.rs"]
mod types;
#[path = "claude_custody_worker_termination.rs"]
mod worker_termination;
#[path = "claude_custody_runtime_retirement.rs"]
#[cfg(target_os = "macos")]
mod runtime_retirement;
#[cfg(target_os = "macos")]
pub use runtime_retirement::settle_runtime_retirement;
pub use context::ManifestAuthorityUnavailable;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
include!("reboot_test_support_wiring.rs");
pub use types::CustodyLedger;
pub use types::{CustodySpec, DurableProcessIdentity};
pub use worker_termination::{
    WorkerTerminationClaim, begin_worker_termination, settle_worker_termination,
};

pub const LEDGER_FILE: &str = "claude-custody.json";
#[cfg(test)]
const SETTINGS_PARENT_MISSING: &str = "Claude custody settings parent is unavailable";
pub fn path(settings_directory: &Path) -> Result<PathBuf> {
    match context::ledger_path_for_settings(settings_directory) {
        Ok(path) => Ok(path),
        #[cfg(test)]
        Err(_) if !context::settings_has_managed_shape(settings_directory) => {
            Ok(settings_directory
                .parent()
                .context(SETTINGS_PARENT_MISSING)?
                .join(LEDGER_FILE))
        }
        Err(error) => Err(error),
    }
}

pub fn spec(
    invocation_id: Uuid,
    target: crate::process::ProcessIdentity,
    authority: &crate::claude_hooks::ClaudeCustodyAuthority,
    primary: crate::process::ProcessIdentity,
    standby: crate::process::ProcessIdentity,
) -> Result<CustodySpec> {
    let current_boot = crate::process::boot_id();
    ensure!(
        crate::process::boot_identity_relation(&current_boot, &current_boot)
            == crate::process::BootIdentityRelation::Same,
        "Claude custody requires a stable boot identity"
    );
    let worker = crate::process::ProcessIdentity {
        pid: authority.worker.pid,
        start_time: authority.worker.start_time,
    };
    Ok(CustodySpec {
        invocation_id,
        target: DurableProcessIdentity::new(current_boot.clone(), target),
        worker: DurableProcessIdentity::new(authority.worker_boot_id.clone(), worker),
        guardians: [
            DurableProcessIdentity::new(current_boot.clone(), primary),
            DurableProcessIdentity::new(current_boot, standby),
        ],
    })
}

pub fn arm(path: &Path, spec: &CustodySpec) -> Result<()> {
    persistence::update(path, |ledger| state::arm(ledger, spec))
}

pub fn activate(path: &Path, spec: &CustodySpec) -> Result<()> {
    persistence::update(path, |ledger| state::activate(ledger, spec))
}

pub fn has_active_startup_receipt(
    namespace: &crate::namespace_lease::NamespaceLease,
    ledger_path: &Path,
    boot_id: &str,
    target: crate::process::ProcessIdentity,
    worker: crate::process::ProcessIdentity,
) -> Result<bool> {
    context::resolve_ledger(ledger_path)?;
    if namespace.file_is_missing(ledger_path)? {
        return Ok(false);
    }
    let (ledger, identity): (CustodyLedger, _) = namespace.read_json_with_identity(ledger_path)?;
    ensure!(
        identity.length() <= types::MAX_LEDGER_BYTES,
        persistence::LEDGER_TOO_LARGE
    );
    persistence::validate(&ledger)?;
    namespace.verify()?;
    Ok(state::has_active_startup_receipt(
        &ledger, boot_id, target, worker,
    ))
}

pub fn mark_containing(path: &Path, spec: &CustodySpec) -> Result<()> {
    persistence::update(path, |ledger| {
        state::transition(ledger, spec, types::CustodyState::Containing)
    })
}

pub fn mark_settled(path: &Path, spec: &CustodySpec) -> Result<()> {
    persistence::update(path, |ledger| {
        state::transition(ledger, spec, types::CustodyState::Settled)
    })
}

pub fn mark_indeterminate(path: &Path, spec: &CustodySpec) -> Result<()> {
    persistence::update(path, |ledger| {
        state::transition(ledger, spec, types::CustodyState::Indeterminate)
    })
}

pub fn admit(settings_directory: &Path) -> Result<()> {
    let path = path(settings_directory)?;
    let blocked = persistence::update(&path, state::prepare_admission)?;
    ensure!(blocked.is_empty(), "{}", blocked.join("; "));
    Ok(())
}

#[cfg(test)]
pub(crate) use reboot::persist_previous_boot_for_test;
pub use reboot_validation::source_before_reboot_migration;

#[cfg(any(test, feature = "adversarial-test-hooks"))]
include!("../test_support/observation_api.rs");
#[cfg(any(test, feature = "adversarial-test-hooks"))]
include!("../test_support/managed_settings_fixture.rs");

#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../test_support/fence.rs"]
mod fence_test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_custody_fence_tests.rs"]
mod fence_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_custody_participant_wait_tests.rs"]
mod participant_wait_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_custody_reboot_tests.rs"]
mod reboot_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_custody_startup_receipt_tests.rs"]
mod startup_receipt_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_custody_tests.rs"]
mod tests;
