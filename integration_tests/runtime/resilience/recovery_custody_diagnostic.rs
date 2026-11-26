use super::*;
use yolo_runtime::pty_custody::CustodyDurableRecord;

const UNREADABLE_STATE: &str = "unreadable";

pub(super) fn capture(harness: &Harness, before: &RuntimeRecord) -> String {
    let paths = harness.paths();
    let manifest = read_json::<yolo_runtime::TerminalManifest>(&paths.manifest(before.terminal_id))
        .map(|manifest| format!("{:?}", manifest.recovery_state))
        .unwrap_or_else(|_| UNREADABLE_STATE.to_owned());
    let Ok(custody) = read_json::<CustodyDurableRecord>(&paths.pty_custody(before.terminal_id))
    else {
        return format!("durable_state={manifest} custody={UNREADABLE_STATE}");
    };
    let matches = custody.runtime.matches_record(before);
    let valid = custody.is_valid();
    let current = read_json::<RuntimeRecord>(&paths.runtime_record(before.terminal_id)).ok();
    let record_unchanged = current.as_ref() == Some(before);
    let controller_generation = current.as_ref().map(|record| record.controller_generation);
    let checkpoint_ready = current.as_ref().and_then(|record| {
        yolo_runtime::worker::worker_startup_checkpoint_allows_lifecycle_mutation(
            &paths,
            before.terminal_id,
            Some(record),
        )
        .ok()
    });
    let worker = process::observe_process_identity(before.worker_pid, before.worker_start_time);
    let child = process::observe_process_identity(before.child_pid, before.child_start_time);
    let child_writer = (matches && valid)
        .then(|| writer_observation(&custody, before.child_pid, before.child_start_time));
    let guardian = (matches && valid)
        .then(|| {
            custody
                .prepared_guardian_candidate()
                .or_else(|| custody.predecessor_lease())
                .and_then(|lease| lease.guardian)
                .map(|guardian| {
                    let identity = guardian.process();
                    (
                        identity.pid,
                        identity.start_time,
                        process::observe_process_identity(identity.pid, identity.start_time),
                    )
                })
        })
        .flatten();
    format!(
        "durable_state={manifest} valid={valid} runtime_matches={matches} armed={} retired={} worker={worker:?} child={child:?} child_writer={child_writer:?} guardian={guardian:?} runtime_record_unchanged={record_unchanged} controller_generation={controller_generation:?} checkpoint_ready={checkpoint_ready:?}",
        custody.armed_lease().is_some(),
        custody.retired_lease().is_some(),
    )
}

#[derive(Debug)]
enum WriterObservation {
    NoActiveLease,
    NoProcessCapability,
    ProcessNotCurrent,
    ProcessChanged,
    Present,
    NotObserved,
    Unavailable,
    #[cfg(not(target_os = "macos"))]
    NotApplicable,
}

fn writer_observation(
    custody: &CustodyDurableRecord,
    pid: u32,
    start_time: u64,
) -> WriterObservation {
    #[cfg(target_os = "macos")]
    {
        let Some(lease) = custody
            .armed_lease()
            .or_else(|| custody.predecessor_lease())
        else {
            return WriterObservation::NoActiveLease;
        };
        let yolo_runtime::pty_custody::CustodyDescriptorSet::MacosProcessCapability {
            writer_handle,
            ..
        } = lease.descriptor_set
        else {
            return WriterObservation::NoProcessCapability;
        };
        if process::observe_process_identity(pid, start_time)
            != process::ProcessIdentityObservation::Matching
        {
            return WriterObservation::ProcessNotCurrent;
        }
        let observed = process::process_holds_capability_for_test(pid, writer_handle);
        if process::observe_process_identity(pid, start_time)
            != process::ProcessIdentityObservation::Matching
        {
            return WriterObservation::ProcessChanged;
        }
        match observed {
            Ok(true) => WriterObservation::Present,
            Ok(false) => WriterObservation::NotObserved,
            Err(_) => WriterObservation::Unavailable,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (custody, pid, start_time);
        WriterObservation::NotApplicable
    }
}
