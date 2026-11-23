use super::ObservedProcess;
use crate::process::ProcessIdentity;
use anyhow::{Context, Result};
use std::io::{Read, Write};
use tokio::io::{AsyncRead, AsyncReadExt};

#[path = "process_snapshot_bounded.rs"]
mod bounded;
#[path = "process_snapshot_decode.rs"]
mod decode;
#[path = "process_snapshot_encode.rs"]
mod encode;
#[path = "process_snapshot_limits.rs"]
mod limits;
#[path = "process_snapshot_receipt.rs"]
mod receipt;
#[path = "process_snapshot_validation.rs"]
mod validation;
#[path = "process_snapshot_validation_budget.rs"]
mod validation_budget;
#[path = "process_snapshot_wire.rs"]
mod wire;
use wire::{ProcessSnapshotEnvelope, ProcessSnapshotWireOutcome};

use limits::{
    MAX_PROCESS_SNAPSHOT_ERROR_BYTES, PROCESS_SNAPSHOT_ERROR_FALLBACK,
    PROCESS_SNAPSHOT_SCHEMA_VERSION,
};
const PROCESS_SNAPSHOT_PARENT_LOST_EXIT_CODE: i32 = 1;
const PROCESS_SNAPSHOT_PARENT_WATCH_START: &str = "start process snapshot parent watchdog";
const PROCESS_SNAPSHOT_PARENT_WATCH_THREAD: &str = "yolo-process-snapshot-parent-watch";
const PROCESS_SNAPSHOT_WATCH_BYTES: usize = 1;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
const PROCESS_SNAPSHOT_TEST_FAILURE: &str = "injected process snapshot failure after frame";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
const PROCESS_SNAPSHOT_TEST_PRE_FRAME_FAILURE: &str =
    "injected process snapshot failure before frame";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
const PROCESS_SNAPSHOT_TEST_REPORTED_FAILURE: &str = "injected process snapshot reported failure";

#[derive(Clone, Copy, Eq, PartialEq)]
enum ProcessSnapshotTestStage {
    BeforeFrame,
    AfterFrame,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ProcessSnapshotOutcome {
    Stable(Vec<ObservedProcess>),
    RootChanged,
    Failed(String),
}

#[derive(Debug, Eq, PartialEq)]
pub struct ProcessSnapshotReceipt {
    pub outcome: ProcessSnapshotOutcome,
    pub started_at_ms: i64,
    pub completed_at_ms: i64,
}

pub fn write_system_process_snapshot(
    root: ProcessIdentity,
    environment_keys: &[String],
    mut writer: impl Write,
) -> Result<()> {
    anyhow::ensure!(
        validation::valid_platform_pid(root.pid) && root.start_time != 0,
        super::constants::process_error::SNAPSHOT_INVALID
    );
    apply_test_behavior(ProcessSnapshotTestStage::BeforeFrame)?;
    let started_at_ms = crate::model::now_ms();
    let outcome = injected_test_outcome()
        .unwrap_or_else(|| encode::observe(root, environment_keys));
    let completed_at_ms = crate::model::now_ms().max(started_at_ms);
    crate::protocol::write_frame_blocking(
        &mut writer,
        &ProcessSnapshotEnvelope {
            schema_version: PROCESS_SNAPSHOT_SCHEMA_VERSION,
            root_pid: root.pid,
            root_start_time: root.start_time,
            started_at_ms,
            completed_at_ms,
            outcome,
        },
    )?;
    writer.flush()?;
    apply_test_behavior(ProcessSnapshotTestStage::AfterFrame)?;
    Ok(())
}

pub fn start_process_snapshot_parent_watch() -> Result<()> {
    std::thread::Builder::new()
        .name(PROCESS_SNAPSHOT_PARENT_WATCH_THREAD.to_owned())
        .spawn(|| {
            let mut input = std::io::stdin().lock();
            let mut byte = [0_u8; PROCESS_SNAPSHOT_WATCH_BYTES];
            loop {
                match input.read(&mut byte) {
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    _ => std::process::exit(PROCESS_SNAPSHOT_PARENT_LOST_EXIT_CODE),
                }
            }
        })
        .context(PROCESS_SNAPSHOT_PARENT_WATCH_START)?;
    Ok(())
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
fn apply_test_behavior(stage: ProcessSnapshotTestStage) -> Result<()> {
    use super::process_snapshot_fault as test;
    let behavior = std::env::var(test::BEHAVIOR_ENV).ok();
    if stage == ProcessSnapshotTestStage::AfterFrame
        && behavior.as_deref() == Some(test::FAIL_AFTER_FRAME)
    {
        anyhow::bail!(PROCESS_SNAPSHOT_TEST_FAILURE);
    }
    if stage == ProcessSnapshotTestStage::BeforeFrame
        && behavior.as_deref() == Some(test::FAIL_BEFORE_FRAME)
    {
        anyhow::bail!(PROCESS_SNAPSHOT_TEST_PRE_FRAME_FAILURE);
    }
    let hangs = matches!(
        (stage, behavior.as_deref()),
        (
            ProcessSnapshotTestStage::BeforeFrame,
            Some(test::HANG_BEFORE_FRAME)
        ) | (
            ProcessSnapshotTestStage::AfterFrame,
            Some(test::HANG_AFTER_FRAME)
        )
    );
    while hangs {
        std::thread::park();
    }
    Ok(())
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
fn injected_test_outcome() -> Option<ProcessSnapshotWireOutcome> {
    (std::env::var(super::process_snapshot_fault::BEHAVIOR_ENV).as_deref()
        == Ok(super::process_snapshot_fault::REPORT_FAILURE))
    .then(|| ProcessSnapshotWireOutcome::Failed {
        detail: PROCESS_SNAPSHOT_TEST_REPORTED_FAILURE.to_owned(),
    })
}

#[cfg(not(any(test, feature = "adversarial-test-hooks")))]
fn apply_test_behavior(_: ProcessSnapshotTestStage) -> Result<()> {
    Ok(())
}

#[cfg(not(any(test, feature = "adversarial-test-hooks")))]
fn injected_test_outcome() -> Option<ProcessSnapshotWireOutcome> {
    None
}

pub async fn read_process_snapshot(
    reader: &mut (impl AsyncRead + Unpin),
    root: ProcessIdentity,
    environment_keys: Vec<String>,
) -> Result<ProcessSnapshotReceipt> {
    let payload = decode::read_payload(reader).await?;
    let mut trailing = [0_u8; 1];
    let count = reader.read(&mut trailing).await?;
    anyhow::ensure!(
        count == 0,
        super::constants::process_error::SNAPSHOT_INVALID
    );
    receipt::decode(payload, root, environment_keys).await
}

fn bounded_error(mut detail: String) -> String {
    if detail.is_empty() {
        return PROCESS_SNAPSHOT_ERROR_FALLBACK.to_owned();
    }
    if detail.len() > MAX_PROCESS_SNAPSHOT_ERROR_BYTES {
        let mut end = MAX_PROCESS_SNAPSHOT_ERROR_BYTES;
        while !detail.is_char_boundary(end) {
            end -= 1;
        }
        detail.truncate(end);
    }
    detail
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_snapshot_error_tests.rs"]
mod error_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_snapshot_test_support.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_snapshot_tests.rs"]
mod tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/process_snapshot_validation_tests.rs"]
mod validation_tests;
