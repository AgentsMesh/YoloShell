use super::*;
use crate::model::{
    BootstrapDelivery, CompletedAttachmentView, ExitRecord, RecoveryState, RuntimeView,
    TerminalManifest,
};
use std::path::PathBuf;
use uuid::Uuid;

const TEST_PROCESS_ID: u32 = 1;
const TEST_START_TIME: u64 = 1;
const TEST_TIMESTAMP_MS: i64 = 1;
const TEST_OUTPUT_SEQUENCE: u64 = 3;
const TEST_NAME: &str = "completed snapshot";
const TEST_COMMAND: &str = "shell";
const TEST_CWD: &str = "/tmp/yolo-completed-snapshot";

#[test]
fn exact_completed_view_returns_its_attested_barrier() {
    let view = completed_view();

    assert_eq!(expected_barrier(&view).unwrap(), Some(TEST_OUTPUT_SEQUENCE));
    let mut active = view;
    active.completed_attachment = None;
    assert_eq!(expected_barrier(&active).unwrap(), None);
}

#[test]
fn inconsistent_completed_views_fail_closed() {
    let mut archived = completed_view();
    archived.manifest.archived = true;
    assert_mismatch(&archived);
    let mut wrong_marker = completed_view();
    wrong_marker
        .completed_attachment
        .as_mut()
        .unwrap()
        .runtime_epoch = Uuid::new_v4();
    assert_mismatch(&wrong_marker);
    let mut wrong_status = completed_view();
    wrong_status.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Running;
    assert_mismatch(&wrong_status);
    let mut wrong_exit = completed_view();
    wrong_exit
        .runtime
        .as_mut()
        .unwrap()
        .exit
        .as_mut()
        .unwrap()
        .runtime_epoch = Uuid::new_v4();
    assert_mismatch(&wrong_exit);
    let mut wrong_reason = completed_view();
    wrong_reason
        .runtime
        .as_mut()
        .unwrap()
        .exit
        .as_mut()
        .unwrap()
        .reason = ExitReason::Lost;
    assert_mismatch(&wrong_reason);
    let mut wrong_sequence = completed_view();
    wrong_sequence
        .runtime
        .as_mut()
        .unwrap()
        .last_output_sequence = u64::default();
    assert_mismatch(&wrong_sequence);
}

fn assert_mismatch(view: &TerminalView) {
    assert!(matches!(
        expected_barrier(view),
        Err(ProtocolError::AttachmentMismatch)
    ));
}

fn completed_view() -> TerminalView {
    let terminal_id = Uuid::new_v4();
    let runtime_epoch = Uuid::new_v4();
    let worker_id = Uuid::new_v4();
    let exit = ExitRecord {
        runtime_epoch,
        code: Some(i32::default()),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: TEST_TIMESTAMP_MS,
        last_output_sequence: TEST_OUTPUT_SEQUENCE,
    };
    TerminalView {
        manifest: TerminalManifest {
            schema_version: yolo_runtime::constants::SESSION_MANIFEST_SCHEMA_VERSION,
            terminal_id: terminal_id,
            name: TEST_NAME.into(),
            command: TEST_COMMAND.into(),
            args: Vec::new(),
            cwd: PathBuf::from(TEST_CWD),
            cols: crate::model::DEFAULT_COLS,
            rows: crate::model::DEFAULT_ROWS,
            created_at_ms: TEST_TIMESTAMP_MS,
            updated_at_ms: TEST_TIMESTAMP_MS,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch,
            worker_id,
            worker_pid: TEST_PROCESS_ID,
            worker_start_time: TEST_START_TIME,
            child_pid: TEST_PROCESS_ID,
            child_start_time: TEST_START_TIME,
            status: TerminalRuntimeStatus::Exited,
            last_output_sequence: TEST_OUTPUT_SEQUENCE,
            exit: Some(exit),
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: None,
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: Some(CompletedAttachmentView { runtime_epoch }),
    }
}
