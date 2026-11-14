use anyhow::Context;
use std::sync::mpsc::{Receiver, Sender};

const INTAKE_OCCURRENCE_MISSING: &str = "Codex intake occurrence is missing for fault injection";
const INTAKE_CLEANUP_GATE_ENTER_FAILED: &str = "Codex intake cleanup gate could not report entry";
const INTAKE_CLEANUP_GATE_RELEASE_FAILED: &str = "Codex intake cleanup gate release was lost";

pub fn fail_next_intake_remove_sync(
    intake: &super::CodexHookIntake,
    occurrence: &crate::agent::hooks::HookEnvelope,
) -> anyhow::Result<std::path::PathBuf> {
    let durable = intake
        .find_exact_occurrence(occurrence)?
        .context(INTAKE_OCCURRENCE_MISSING)?;
    let identity = durable.durable_identity()?;
    let path = intake.path(identity.file_name());
    let source = crate::paths::private_file::PrivateFileEntry::from_directory(
        intake.directory().try_clone()?,
        &path,
    )?;
    let isolation = source.unlink_isolation_entry()?.path().to_path_buf();
    super::intake_storage::set_before_remove_hook(path, || {
        crate::paths::test_support::fail_next_directory_sync();
    });
    Ok(isolation)
}

pub fn gate_intake_remove_before_sync(
    intake: &super::CodexHookIntake,
    occurrence: &crate::agent::hooks::HookEnvelope,
    entered: Sender<()>,
    release: Receiver<()>,
) -> anyhow::Result<(std::path::PathBuf, impl Drop)> {
    let durable = intake
        .find_exact_occurrence(occurrence)?
        .context(INTAKE_OCCURRENCE_MISSING)?;
    let identity = durable.durable_identity()?;
    let path = intake.path(identity.file_name());
    let source = crate::paths::private_file::PrivateFileEntry::from_directory(
        intake.directory().try_clone()?,
        &path,
    )?;
    let isolation = source.unlink_isolation_entry()?.path().to_path_buf();
    let guard =
        crate::paths::test_support::install_after_unlink_before_sync_hook(&source, move || {
            entered
                .send(())
                .map_err(|_| anyhow::anyhow!(INTAKE_CLEANUP_GATE_ENTER_FAILED))?;
            release
                .recv()
                .map_err(|_| anyhow::anyhow!(INTAKE_CLEANUP_GATE_RELEASE_FAILED))?;
            Ok(())
        });
    Ok((isolation, guard))
}
