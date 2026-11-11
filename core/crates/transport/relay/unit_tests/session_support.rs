use crate::model::{
    ExitReason, ExitRecord, HostStatus, PROTOCOL_VERSION, RecoveryState, RuntimeView,
    TerminalManifest, TerminalRuntimeStatus, TerminalView, TerminalSnapshot, WriterLease,
};
use crate::protocol::{EventEnvelope, event};
use crate::relay::test_support::Reply;
use crate::worker::AttachResult;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};
use tokio::sync::mpsc;
use uuid::Uuid;

pub(crate) const GENERATION: u64 = 17;
pub(crate) const HOLDER: &str = "session-flow-holder";
pub(crate) const OTHER_HOLDER: &str = "other-holder";
pub(crate) const TEST_RUNTIME_EPOCH_VALUE: u128 = 1;
const CLOSED_RESIZE_CHANNEL_CAPACITY: usize = 1;

pub(crate) struct PendingInput;

impl AsyncRead for PendingInput {
    fn poll_read(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        _buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Pending
    }
}

pub(crate) fn options(terminal_id: Uuid) -> crate::relay::RelayOptions {
    crate::relay::RelayOptions::new(terminal_id, HOLDER).unwrap()
}

pub(crate) fn closed_resize_channel() -> mpsc::Receiver<crate::relay::TerminalSize> {
    let (sender, receiver) = mpsc::channel(CLOSED_RESIZE_CHANNEL_CAPACITY);
    drop(sender);
    receiver
}

pub(crate) fn owned_prefix(terminal_id: Uuid) -> Vec<Reply> {
    owned_with_attachment(terminal_id, Reply::attachment(empty_attach(), Vec::new()))
}

pub(crate) fn owned_with_attachment(terminal_id: Uuid, attachment: Reply) -> Vec<Reply> {
    let mut replies = attached_prefix(terminal_id, attachment);
    replies.push(Reply::Lease(GENERATION));
    replies
}

pub(crate) fn attached_prefix(terminal_id: Uuid, attachment: Reply) -> Vec<Reply> {
    let mut replies = reconnect_prefix(terminal_id);
    replies.push(attachment);
    replies
}

pub(crate) fn reconnect_prefix(terminal_id: Uuid) -> Vec<Reply> {
    vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
    ]
}

pub(crate) fn read_only_prefix(terminal_id: Uuid) -> Vec<Reply> {
    vec![
        Reply::value(&host_status()),
        Reply::value(&running_session(terminal_id)),
        Reply::attachment(empty_attach(), Vec::new()),
        Reply::error(crate::relay::constants::error_code::HELD_BY_OTHER),
        Reply::value(&Some(other_lease())),
    ]
}

pub(crate) fn empty_attach() -> AttachResult {
    empty_attach_for(test_runtime_epoch())
}

pub(crate) fn empty_attach_for(runtime_epoch: Uuid) -> AttachResult {
    AttachResult {
        runtime_epoch,
        snapshot: None,
        chunks: Vec::new(),
        next_sequence: 1,
        resynchronized: false,
    }
}

pub(crate) fn snapshot_attach(data: &[u8]) -> AttachResult {
    AttachResult {
        runtime_epoch: test_runtime_epoch(),
        snapshot: Some(TerminalSnapshot {
            runtime_epoch: test_runtime_epoch(),
            barrier_sequence: 0,
            cols: 80,
            rows: 24,
            data: data.to_vec(),
        }),
        chunks: Vec::new(),
        next_sequence: 1,
        resynchronized: true,
    }
}

pub(crate) fn session_event(event_type: &str, terminal_id: Uuid) -> EventEnvelope {
    EventEnvelope::new(
        event_type,
        Some(terminal_id.to_string()),
        Some(test_runtime_epoch().to_string()),
        &(),
    )
    .unwrap()
}

include!("session_support_exit.rs");

pub(crate) fn other_lease() -> WriterLease {
    WriterLease {
        holder_id: OTHER_HOLDER.into(),
        controller_instance_id: Some("other-controller".into()),
        generation: GENERATION + 1,
        expires_at_ms: i64::MAX,
        ttl_ms: crate::lease::constants::DEFAULT_TTL_MS,
        renew_after_ms: crate::lease::constants::DEFAULT_TTL_MS
            / crate::lease::constants::RENEW_AFTER_DIVISOR,
    }
}

pub(crate) fn host_status() -> HostStatus {
    HostStatus {
        protocol_version: PROTOCOL_VERSION,
        boot_id: "test-boot".into(),
        host_pid: std::process::id(),
        controller_generation: 1,
        home: PathBuf::from("/tmp/yolo-test"),
        capabilities: Vec::new(),
    }
}

pub(crate) fn running_session(terminal_id: Uuid) -> TerminalView {
    running_session_for_epoch(terminal_id, test_runtime_epoch())
}

pub(crate) fn running_session_for_epoch(terminal_id: Uuid, runtime_epoch: Uuid) -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: 1,
            terminal_id: terminal_id,
            name: "flow-session".into(),
            command: "/bin/sh".into(),
            args: Vec::new(),
            cwd: PathBuf::from("/tmp"),
            cols: 80,
            rows: 24,
            created_at_ms: 1,
            updated_at_ms: 1,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: Some(RuntimeView {
            presentation: Default::default(),
            runtime_epoch,
            worker_id: Uuid::new_v4(),
            worker_pid: std::process::id(),
            worker_start_time: u64::from(std::process::id()),
            child_pid: std::process::id(),
            child_start_time: u64::from(std::process::id()),
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: 0,
            exit: None,
            bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}

pub(crate) fn test_runtime_epoch() -> Uuid {
    Uuid::from_u128(TEST_RUNTIME_EPOCH_VALUE)
}
