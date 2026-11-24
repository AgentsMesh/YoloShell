use crc32fast::Hasher;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

use crate::model::{OutputChunk, TerminalSnapshot};
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::PrivateFileEntry;

#[path = "append.rs"]
mod append;
#[path = "append_batch.rs"]
mod append_batch;
#[path = "commit_cursor.rs"]
mod commit_cursor;
#[path = "commit_cursor_format.rs"]
mod commit_cursor_format;
#[path = "compact.rs"]
mod compact;
#[path = "constants.rs"]
mod constants;
#[path = "header.rs"]
mod header;
#[path = "open.rs"]
mod open;
#[path = "read.rs"]
mod read;
#[path = "reconcile.rs"]
mod reconcile;
#[path = "reconcile_retained.rs"]
mod reconcile_retained;
#[path = "reconcile_retained_batch.rs"]
mod reconcile_retained_batch;
#[path = "reconcile_retained_confirm.rs"]
mod reconcile_retained_confirm;
#[path = "reconcile_retained_outcome.rs"]
mod reconcile_retained_outcome;
#[path = "record.rs"]
mod record;
#[path = "retained_evidence.rs"]
mod retained_evidence;
#[path = "scan.rs"]
mod scan;
#[path = "snapshot.rs"]
mod snapshot;
#[path = "snapshot_format.rs"]
mod snapshot_format;
#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/tests_retained_hooks.rs"]
mod tests_retained_hooks;

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/append.rs"]
mod append_test_support;
#[cfg(any(test, feature = "runtime-component-tests"))]
pub use append_test_support::AppendStage;
#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../test_support/runtime_api.rs"]
mod runtime_test_support;
use commit_cursor::{CommitCursor, LoadedCursor};
use constants::*;
use header::JournalHeader;
pub use reconcile::AppendAttempt;
pub use reconcile_retained_outcome::{RetainedAppendBatchOutcome, RetainedAppendOutcome};
use record::{checksum_payload, read_payload, write_record};
pub use retained_evidence::RetainedJournalEvidence;
#[cfg(any(test, feature = "yolo-test-support"))]
#[path = "../test_support/retained_cursor.rs"]
mod retained_cursor_test_support;
#[cfg(any(test, feature = "yolo-test-support"))]
pub use retained_cursor_test_support::retained_cursor_path_for_test;
pub use snapshot::write_snapshot_retained;
pub use snapshot::{read_snapshot, write_snapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordKind {
    Output,
    Cursor,
}

#[derive(Debug, Clone)]
struct RecordIndex {
    runtime_epoch: Uuid,
    sequence: u64,
    payload_offset: u64,
    payload_len: u32,
    checksum: u32,
}

#[derive(Debug)]
pub struct Journal {
    entry: PrivateFileEntry,
    file: File,
    identity: FileIdentity,
    header: JournalHeader,
    commit_cursor: CommitCursor,
    runtime_epoch: Uuid,
    records: VecDeque<RecordIndex>,
    retained_from_sequence: u64,
    last_sequence: u64,
    disk_bytes: u64,
    max_disk_bytes: u64,
    write_healthy: bool,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    append_fault: Option<append_test_support::AppendFault>,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    reconcile_faults: usize,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    retained_reconcile_faults: usize,
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal record {sequence} is corrupt")]
    Corrupt { sequence: u64 },
    #[error("journal format is invalid")]
    InvalidFormat,
    #[error("journal payload is too large")]
    PayloadTooLarge,
    #[error("journal disk limit would be exceeded")]
    DiskLimitExceeded,
    #[error("journal sequence is exhausted")]
    SequenceExhausted,
    #[error("journal cursor publication revision is exhausted")]
    CursorRevisionExhausted,
    #[error("journal write state is uncertain; reopen is required")]
    WriteStateUncertain,
    #[error("durable journal validation failed: {0}")]
    Durability(#[from] anyhow::Error),
    #[error("snapshot encoding failed: {0}")]
    SnapshotEncode(rmp_serde::encode::Error),
    #[error("snapshot decoding failed: {0}")]
    SnapshotDecode(rmp_serde::decode::Error),
}

impl JournalError {
    pub fn retryable_without_reconcile(&self) -> bool {
        matches!(self, Self::Io(_) | Self::Durability(_))
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
include!("../unit_tests/module_wiring.rs");
