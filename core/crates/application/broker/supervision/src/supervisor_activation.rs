use super::*;

pub(super) fn activate_child(
    child: &mut DetachedChildCustody,
    control: &mut ActivationSender,
    input: Option<std::process::ChildStdin>,
    private_input: Option<zeroize::Zeroizing<Vec<u8>>>,
    record: &mut SupervisionRecord,
    paths: &SupervisionPaths,
    expected_host: &str,
    expected_revision: &str,
    deadline: std::time::Instant,
) -> Result<()> {
    record.activate(
        child
            .identity()
            .context(crate::supervisor_process::SPAWN_CONTEXT)?,
    )?;
    paths.write_record(record)?;
    control.activate()?;
    if let Some(private_input) = private_input {
        yolo_process::write_detached_child_private_input(
            input.context(STDIN_CONTEXT)?,
            private_input.as_slice(),
            deadline,
        )?;
    }
    let readiness = control.read_readiness(deadline)?;
    validate_broker_config_revision(&readiness.config_revision)?;
    validate_broker_host_id(&readiness.host_id)?;
    ensure!(!readiness.instance_id.is_nil(), READINESS_INSTANCE_INVALID);
    ensure!(
        readiness.config_revision == expected_revision,
        READINESS_REVISION_MISMATCH
    );
    ensure!(readiness.host_id == expected_host, READINESS_HOST_MISMATCH);
    record.ready(&readiness)?;
    paths.write_record(record)
}
