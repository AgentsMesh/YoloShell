use super::types::{CustodyLedger, CustodySpec, DurableProcessIdentity};
use anyhow::{Result, ensure};

const BOOT_ID_INVALID: &str = "Claude custody reboot identity is invalid";
const WORKER_IDENTITY_INVALID: &str = "Claude custody reboot worker identity is invalid";

pub fn source_before_reboot_migration(
    target: &CustodyLedger,
    runtime: &crate::model::RuntimeRecord,
    source_boot_id: &str,
    target_boot_id: &str,
) -> Result<CustodyLedger> {
    super::persistence::validate(target)?;
    let mut source = target.clone();
    for record in &mut source.records {
        restore_spec(&mut record.spec, runtime, source_boot_id, target_boot_id)?;
    }
    super::persistence::validate(&source)?;
    Ok(source)
}

fn restore_spec(
    spec: &mut CustodySpec,
    runtime: &crate::model::RuntimeRecord,
    source_boot_id: &str,
    target_boot_id: &str,
) -> Result<()> {
    ensure!(
        identities(spec).all(|identity| same_boot(&identity.boot_id, target_boot_id)),
        BOOT_ID_INVALID
    );
    ensure!(
        spec.worker.pid == runtime.worker_pid
            && spec.worker.start_time == runtime.worker_start_time,
        WORKER_IDENTITY_INVALID
    );
    for identity in identities_mut(spec) {
        identity.boot_id = source_boot_id.to_owned();
    }
    Ok(())
}

fn same_boot(value: &str, expected: &str) -> bool {
    crate::process::boot_identity_relation(value, expected)
        == crate::process::BootIdentityRelation::Same
}

fn identities(spec: &CustodySpec) -> impl Iterator<Item = &DurableProcessIdentity> {
    std::iter::once(&spec.target)
        .chain(std::iter::once(&spec.worker))
        .chain(spec.guardians.iter())
}

fn identities_mut(spec: &mut CustodySpec) -> impl Iterator<Item = &mut DurableProcessIdentity> {
    std::iter::once(&mut spec.target)
        .chain(std::iter::once(&mut spec.worker))
        .chain(spec.guardians.iter_mut())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/claude_custody_reboot_validation_tests.rs"]
mod tests;
