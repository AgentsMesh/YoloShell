use super::startup_guard::StartupChildGuard;
use crate::model::{BootstrapDelivery, RuntimeRecord};
use crate::paths::{atomic_write_json, read_json};
use crate::process::process_identity_matches;
use anyhow::{Context, Result, anyhow, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

pub const SPEC_ENVIRONMENT: &str = "YOLO_TEST_STARTUP_HANDOFF_SPEC";
pub const READY_MARKER_FILE: &str = "ready.json";
pub const FAILURE_MARKER_FILE: &str = "failure.json";
pub const HANDOFF_MARKER_FILE: &str = "handoff.json";
pub const RELEASE_MARKER_FILE: &str = "release.json";
pub const REAPED_MARKER_FILE: &str = "reaped.json";
pub const COMPLETION_ACK_MARKER_FILE: &str = "completion-ack.json";
pub const COMPLETED_MARKER_FILE: &str = "completed.json";
pub const INJECTED_FAILURE_DETAIL: &str = "injected startup custody handoff failure";

const SPEC_UNICODE_DETAIL: &str = "startup handoff test specification is not Unicode";
const SPEC_DECODE_CONTEXT: &str = "decode startup handoff test specification";
const SPEC_DIRECTORY_DETAIL: &str = "startup handoff test directory must be absolute";
const FAILURE_STATE_DETAIL: &str = "startup handoff injection requires a failed bootstrap";
const CHILD_STILL_LIVE_DETAIL: &str = "startup handoff child is still live after settlement";
const FAILURE_MARKER_UNAVAILABLE_CONTEXT: &str = "startup handoff failure marker is unavailable";
const MARKER_FAILURE_DETAIL: &str = "startup handoff test marker operation failed";
const MARKER_POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupHandoffTestSpec {
    pub run_id: Uuid,
    pub directory: PathBuf,
}

impl StartupHandoffTestSpec {
    pub fn marker(&self, file: &str) -> PathBuf {
        self.directory.join(file)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartupHandoffMarker {
    pub run_id: Uuid,
    pub runtime_epoch: Uuid,
    pub worker_pid: u32,
    pub worker_start_time: u64,
    pub child_pid: u32,
    pub child_start_time: u64,
    pub bootstrap_delivery: BootstrapDelivery,
}

impl StartupHandoffMarker {
    fn from_record(spec: &StartupHandoffTestSpec, record: &RuntimeRecord) -> Self {
        Self {
            run_id: spec.run_id,
            runtime_epoch: record.runtime_epoch,
            worker_pid: record.worker_pid,
            worker_start_time: record.worker_start_time,
            child_pid: record.child_pid,
            child_start_time: record.child_start_time,
            bootstrap_delivery: record.bootstrap_delivery,
        }
    }
}

pub(super) fn publish_ready(record: &RuntimeRecord) -> Result<()> {
    publish_record_marker(record, READY_MARKER_FILE)
}

pub(super) fn requested() -> Result<bool> {
    load_spec().map(|spec| spec.is_some())
}

pub(super) fn prepare_failure(record: &RuntimeRecord, child: &mut StartupChildGuard) -> Result<()> {
    let Some(spec) = load_spec()? else {
        return Ok(());
    };
    ensure!(
        record.bootstrap_delivery == BootstrapDelivery::Failed,
        FAILURE_STATE_DETAIL
    );
    let marker = StartupHandoffMarker::from_record(&spec, record);
    atomic_write_json(&spec.marker(FAILURE_MARKER_FILE), &marker)?;
    child.handoff_to_untracked_reaper()
}

pub(super) async fn wait_for_release() -> Result<()> {
    loop {
        let (spec, failure) = load_failure()?.context(FAILURE_MARKER_UNAVAILABLE_CONTEXT)?;
        if marker_matches(&spec, RELEASE_MARKER_FILE, &failure) {
            return Ok(());
        }
        tokio::time::sleep(MARKER_POLL_INTERVAL).await;
    }
}

pub(super) fn publish_handoff() {
    publish_failure_marker(HANDOFF_MARKER_FILE);
}

pub(super) fn complete_settlement() {
    let result = load_failure().and_then(|failure| {
        let Some((spec, marker)) = failure else {
            return Ok(());
        };
        ensure!(
            !process_identity_matches(marker.child_pid, marker.child_start_time),
            CHILD_STILL_LIVE_DETAIL
        );
        atomic_write_json(&spec.marker(REAPED_MARKER_FILE), &marker)?;
        while !marker_matches(&spec, COMPLETION_ACK_MARKER_FILE, &marker) {
            std::thread::sleep(MARKER_POLL_INTERVAL);
        }
        atomic_write_json(&spec.marker(COMPLETED_MARKER_FILE), &marker)
    });
    report_marker_failure(result);
}

fn publish_record_marker(record: &RuntimeRecord, file: &str) -> Result<()> {
    let Some(spec) = load_spec()? else {
        return Ok(());
    };
    atomic_write_json(
        &spec.marker(file),
        &StartupHandoffMarker::from_record(&spec, record),
    )
}

fn publish_failure_marker(file: &str) {
    let result = load_failure().and_then(|failure| match failure {
        Some((spec, marker)) => atomic_write_json(&spec.marker(file), &marker),
        None => Ok(()),
    });
    report_marker_failure(result);
}

fn marker_matches(
    spec: &StartupHandoffTestSpec,
    file: &str,
    expected: &StartupHandoffMarker,
) -> bool {
    read_json::<StartupHandoffMarker>(&spec.marker(file))
        .ok()
        .as_ref()
        == Some(expected)
}

fn load_failure() -> Result<Option<(StartupHandoffTestSpec, StartupHandoffMarker)>> {
    let Some(spec) = load_spec()? else {
        return Ok(None);
    };
    let marker = read_json(&spec.marker(FAILURE_MARKER_FILE))?;
    Ok(Some((spec, marker)))
}

fn load_spec() -> Result<Option<StartupHandoffTestSpec>> {
    let Some(encoded) = std::env::var_os(SPEC_ENVIRONMENT) else {
        return Ok(None);
    };
    let encoded = encoded
        .into_string()
        .map_err(|_| anyhow!(SPEC_UNICODE_DETAIL))?;
    let spec: StartupHandoffTestSpec =
        serde_json::from_str(&encoded).context(SPEC_DECODE_CONTEXT)?;
    ensure!(spec.directory.is_absolute(), SPEC_DIRECTORY_DETAIL);
    Ok(Some(spec))
}

fn report_marker_failure(result: Result<()>) {
    if let Err(error) = result {
        tracing::warn!(%error, MARKER_FAILURE_DETAIL);
    }
}
