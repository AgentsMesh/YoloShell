use super::{CustodyDurableRecord, CustodyRuntimeIdentity};
use crate::model::RuntimeRecord;
use anyhow::{Result, ensure};

const RECORD_INVALID: &str = "PTY custody reboot test record is invalid";
const RUNTIME_MISMATCH: &str = "PTY custody reboot test runtime identity does not match";
const BOOT_MISMATCH: &str = "PTY custody reboot test record belongs to an unexpected boot";

pub fn rewrite_boot_for_test(
    custody: &mut CustodyDurableRecord,
    runtime: &RuntimeRecord,
    previous_boot_id: &str,
) -> Result<()> {
    ensure!(custody.is_valid(), RECORD_INVALID);
    let expected = CustodyRuntimeIdentity::from_record(runtime);
    ensure!(
        matches_non_boot(&custody.runtime, &expected),
        RUNTIME_MISMATCH
    );
    use crate::process::BootIdentityRelation::Same;
    let current =
        crate::process::boot_identity_relation(&custody.runtime.boot_id, &runtime.boot_id);
    let previous =
        crate::process::boot_identity_relation(&custody.runtime.boot_id, previous_boot_id);
    ensure!(current == Same || previous == Same, BOOT_MISMATCH);
    *custody = super::rewrite_boot_for_target(custody, &runtime.boot_id, previous_boot_id)?;
    Ok(())
}

fn matches_non_boot(actual: &CustodyRuntimeIdentity, expected: &CustodyRuntimeIdentity) -> bool {
    actual.terminal_id == expected.terminal_id
        && actual.runtime_epoch == expected.runtime_epoch
        && actual.worker_id == expected.worker_id
        && actual.worker_pid == expected.worker_pid
        && actual.worker_start_time == expected.worker_start_time
        && actual.child_pid == expected.child_pid
        && actual.child_start_time == expected.child_start_time
}
