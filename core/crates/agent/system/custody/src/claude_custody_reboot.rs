use super::CustodySpec;
use anyhow::Result;

const REBOOT_TEST_BOOT_MISMATCH: &str =
    "Claude custody boot identity does not match the reboot test runtime";

#[cfg(test)]
pub(crate) fn persist_previous_boot_for_test(
    session_directory: &std::path::Path,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    let path = session_directory.join(super::LEDGER_FILE);
    if !path.try_exists()? {
        return Ok(());
    }
    super::persistence::update(&path, |ledger| {
        for record in &mut ledger.records {
            rewrite_spec_boot_for_test(&mut record.spec, current_boot_id, previous_boot_id)?;
        }
        Ok(())
    })
}

pub(crate) fn rewrite_spec_boot_for_test(
    spec: &mut CustodySpec,
    current_boot_id: &str,
    previous_boot_id: &str,
) -> Result<()> {
    rewrite_boot(&mut spec.target.boot_id, current_boot_id, previous_boot_id)?;
    rewrite_boot(&mut spec.worker.boot_id, current_boot_id, previous_boot_id)?;
    for guardian in &mut spec.guardians {
        rewrite_boot(&mut guardian.boot_id, current_boot_id, previous_boot_id)?;
    }
    Ok(())
}

fn rewrite_boot(value: &mut String, current: &str, previous: &str) -> Result<()> {
    use crate::process::BootIdentityRelation::{Different, Same, Unknown};
    match crate::process::boot_identity_relation(value, current) {
        Same => *value = previous.to_owned(),
        Unknown => anyhow::bail!(REBOOT_TEST_BOOT_MISMATCH),
        Different => match crate::process::boot_identity_relation(value, previous) {
            Same | Different => {}
            Unknown => anyhow::bail!(REBOOT_TEST_BOOT_MISMATCH),
        },
    }
    Ok(())
}
