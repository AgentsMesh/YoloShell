use super::constants::*;
use super::fixture::{MemoryConnection, connect_memory_with_timeout};
use super::memory_fs::MemoryFilesystem;
use crate::remote::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub async fn fixture(limit: Duration) -> (MemoryConnection, MemoryFilesystem) {
    let filesystem = MemoryFilesystem::default();
    let fixture = connect_memory_with_timeout(false, filesystem.clone(), limit)
        .await
        .unwrap();
    (fixture, filesystem)
}

pub fn authorized_read() -> AuthorizedFileRead {
    FileReadPolicy::standard(vec![PathBuf::from(MEMORY_READ_ROOT)])
        .unwrap()
        .authorize(
            FileReadRequest::new(MEMORY_FILE, SFTP_TEST_READ_OFFSET, SFTP_TEST_READ_LENGTH),
            MEMORY_FILE,
        )
        .unwrap()
}

pub fn short_timeout() -> Duration {
    SHORT_OPERATION_TIMEOUT
}

pub fn normal_timeout() -> Duration {
    NORMAL_OPERATION_TIMEOUT
}

pub fn assert_denied(result: Result<ChannelOutcome<FileReadResult>, RemoteError>) {
    assert!(matches!(result, Err(RemoteError::PathDenied(_))));
}

pub fn assert_transport(result: Result<ChannelOutcome<FileReadResult>, RemoteError>) {
    assert!(matches!(result, Err(RemoteError::Transport(_))));
}

pub fn assert_bounded(started: Instant) {
    assert!(started.elapsed().as_millis() < MAX_OPERATION_ELAPSED_MS);
}

pub async fn assert_sftp_reclaimed(filesystem: &MemoryFilesystem, expected_drop_reclaims: usize) {
    let reclaimed = tokio::time::timeout(SFTP_LIFECYCLE_TIMEOUT, async {
        loop {
            if filesystem.live_handles() == NO_LIVE_HANDLES
                && filesystem.sftp_sessions_started() == EXPECTED_SFTP_SESSIONS
                && filesystem.sftp_sessions_dropped() == EXPECTED_SFTP_SESSIONS
                && filesystem.handles_reclaimed_on_drop() == expected_drop_reclaims
            {
                return;
            }
            tokio::time::sleep(SFTP_LIFECYCLE_POLL_INTERVAL).await;
        }
    })
    .await;
    assert!(reclaimed.is_ok());
}
