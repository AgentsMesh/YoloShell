use super::*;

pub(super) fn wait_for_ready(
    supervisor: &mut yolo_process::DetachedChildCustody,
    options: &ChildLaunch,
    paths: &SupervisionPaths,
    supervisor_identity: ProcessIdentity,
    launcher_identity: ProcessIdentity,
    deadline: std::time::Instant,
) -> Result<yolo_broker_supervision_state::SupervisionRecord> {
    loop {
        anyhow::ensure!(supervisor.try_wait()?.is_none(), BROKER_SUPERVISOR_EXITED);
        match paths.read_record() {
            Ok(record) => {
                record.verify(options, supervisor_identity, launcher_identity)?;
                if record.phase() == SupervisionPhase::Ready {
                    let broker = record.broker_identity().context(BROKER_IDENTITY_MISSING)?;
                    match broker.observe() {
                        ProcessIdentityObservation::Matching => return Ok(record),
                        ProcessIdentityObservation::Gone => {
                            anyhow::bail!(BROKER_IDENTITY_GONE)
                        }
                        ProcessIdentityObservation::Unknown => {}
                    }
                }
            }
            Err(error)
                if yolo_broker_supervision_state::io_kind(&error)
                    == Some(std::io::ErrorKind::NotFound) => {}
            Err(error) => return Err(error),
        }
        let now = std::time::Instant::now();
        anyhow::ensure!(now < deadline, BROKER_SUPERVISION_RECORD_MISSING);
        std::thread::sleep(
            deadline
                .saturating_duration_since(now)
                .min(yolo_remote_broker::BROKER_READY_POLL_INTERVAL),
        );
    }
}

pub(super) fn verified_record(
    paths: &SupervisionPaths,
    options: &ChildLaunch,
    supervisor: ProcessIdentity,
    launcher: ProcessIdentity,
) -> Result<yolo_broker_supervision_state::SupervisionRecord> {
    let record = paths.read_record()?;
    record.verify(options, supervisor, launcher)?;
    Ok(record)
}
