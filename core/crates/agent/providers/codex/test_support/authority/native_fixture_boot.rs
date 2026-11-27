use super::*;

#[path = "native_fixture_constants.rs"]
mod constants;
use constants::*;

pub fn persist_native_previous_boot(
    binding_path: &Path,
    runtime_epoch: uuid::Uuid,
    current_boot: &str,
    previous_boot: &str,
) -> Result<()> {
    ensure!(
        !current_boot.is_empty() && !previous_boot.is_empty() && current_boot != previous_boot,
        INVALID
    );
    let (namespace, store, current, identity) = capture(binding_path)?;
    ensure!(
        current
            .resume_boot_id()
            .is_some_and(|boot| boot == current_boot || boot == previous_boot),
        INVALID
    );
    let mut document = serde_json::to_value(&current)?;
    ensure!(
        document.get(RUNTIME_EPOCH_FIELD) == Some(&serde_json::to_value(runtime_epoch)?),
        INVALID
    );
    let boot = document
        .get_mut(RESUME_IDENTITY_FIELD)
        .and_then(|resume| resume.get_mut(BOOT_ID_FIELD))
        .context(INVALID)?;
    *boot = serde_json::Value::String(previous_boot.to_owned());
    let target: CodexBindingAuthority = serde_json::from_value(document)?;
    target.validate()?;
    target.verify_resume_evidence()?;
    namespace.write_json_expected_with_commit_fence(
        store.path(),
        Some(identity),
        &target,
        || current.verify_resume_evidence(),
    )?;
    store.require_exact(&namespace, &target)
}
