use super::constants::*;
use super::fixture::{connect_memory_with_fs, connect_memory_with_timeout};
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use crate::remote::constants::{MAX_REMOTE_PATH_BYTES, TEST_PATH_FILL};

#[tokio::test]
async fn setup_and_path_encoding_failures_are_bounded() {
    for failure in [FailurePoint::SftpTransport, FailurePoint::SftpInit] {
        let filesystem = MemoryFilesystem::default();
        filesystem.fail_once(failure);
        let fixture = connect_memory_with_timeout(false, filesystem, SHORT_OPERATION_TIMEOUT)
            .await
            .unwrap();
        assert!(
            fixture
                .connection
                .posix_rename(MEMORY_RUNTIME, MEMORY_WRONG_PREFIX_RELEASE)
                .await
                .is_err()
        );
    }
    let fixture = connect_memory_with_fs(false, MemoryFilesystem::default())
        .await
        .unwrap();
    let oversized = TEST_PATH_FILL.repeat(MAX_REMOTE_PATH_BYTES + 1);
    assert!(
        fixture
            .connection
            .posix_rename(&oversized, MEMORY_RUNTIME)
            .await
            .is_err()
    );
}
