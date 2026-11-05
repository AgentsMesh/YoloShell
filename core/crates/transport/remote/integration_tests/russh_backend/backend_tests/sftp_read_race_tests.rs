use super::constants::*;
use super::fixture::connect_memory_with_fs;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use super::sftp_read_support::*;
use crate::remote::constants::RUNTIME_UPLOAD_MODE;
use crate::remote::*;
use std::time::Instant;

#[tokio::test]
async fn path_swaps_before_open_after_open_and_after_read_are_denied() {
    for failure in [
        FailurePoint::SftpSwapBeforeOpen,
        FailurePoint::SftpSwapAfterOpen,
        FailurePoint::SftpSwapAfterRead,
    ] {
        let (fixture, filesystem) = fixture(normal_timeout()).await;
        filesystem.fail_once(failure);
        assert_denied(fixture.connection.read_file(&authorized_read()).await);
        assert_eq!(
            filesystem.target(MEMORY_FILE).as_deref(),
            Some(MEMORY_OUTSIDE)
        );
        assert_eq!(filesystem.close_attempts(), EXPECTED_CLOSE_ATTEMPTS);
        assert_sftp_reclaimed(&filesystem, NO_DROP_RECLAIMED_HANDLES).await;
    }
}

#[tokio::test]
async fn symlinks_and_non_regular_paths_are_rejected_before_open() {
    let filesystem = MemoryFilesystem::default();
    filesystem.insert_link(MEMORY_FILE, MEMORY_FILE);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    assert_denied(fixture.connection.read_file(&authorized_read()).await);
    assert_eq!(filesystem.close_attempts(), NO_CLOSE_ATTEMPTS);
    assert_sftp_reclaimed(&filesystem, NO_DROP_RECLAIMED_HANDLES).await;

    let filesystem = MemoryFilesystem::default();
    filesystem.insert_directory(MEMORY_FILE, RUNTIME_UPLOAD_MODE);
    let fixture = connect_memory_with_fs(false, filesystem.clone())
        .await
        .unwrap();
    assert_denied(fixture.connection.read_file(&authorized_read()).await);
    assert_eq!(filesystem.close_attempts(), NO_CLOSE_ATTEMPTS);
    assert_sftp_reclaimed(&filesystem, NO_DROP_RECLAIMED_HANDLES).await;
}

#[tokio::test]
async fn opened_handle_metadata_drift_is_denied_and_closed() {
    let (fixture, filesystem) = fixture(normal_timeout()).await;
    filesystem.fail_once(FailurePoint::SftpMetadataDriftAfterOpen);
    assert_denied(fixture.connection.read_file(&authorized_read()).await);
    assert_eq!(filesystem.close_attempts(), EXPECTED_CLOSE_ATTEMPTS);
    assert_sftp_reclaimed(&filesystem, NO_DROP_RECLAIMED_HANDLES).await;
}

#[tokio::test]
async fn successful_read_returns_data_only_after_closing_the_handle() {
    let (fixture, filesystem) = fixture(normal_timeout()).await;
    let outcome = fixture
        .connection
        .read_file(&authorized_read())
        .await
        .unwrap();
    assert_eq!(
        outcome.value.data,
        &FILE_CONTENT[..SFTP_TEST_READ_LENGTH as usize]
    );
    assert_eq!(filesystem.close_attempts(), EXPECTED_CLOSE_ATTEMPTS);
    assert_sftp_reclaimed(&filesystem, NO_DROP_RECLAIMED_HANDLES).await;
}

#[tokio::test]
async fn close_failure_and_stall_discard_read_data_with_a_bounded_result() {
    for failure in [FailurePoint::FileClose, FailurePoint::SftpCloseHang] {
        let (fixture, filesystem) = fixture(short_timeout()).await;
        filesystem.fail_once(failure);
        let started = Instant::now();
        assert_transport(fixture.connection.read_file(&authorized_read()).await);
        assert_bounded(started);
        assert!(filesystem.close_attempts() >= EXPECTED_CLOSE_ATTEMPTS);
        assert_sftp_reclaimed(&filesystem, EXPECTED_DROP_RECLAIMED_HANDLES).await;
    }
}

#[tokio::test]
async fn read_and_handle_metadata_stalls_are_bounded_and_close_the_handle() {
    for failure in [FailurePoint::SftpRead, FailurePoint::SftpFstatHang] {
        let (fixture, filesystem) = fixture(short_timeout()).await;
        filesystem.fail_once(failure);
        let started = Instant::now();
        assert_transport(fixture.connection.read_file(&authorized_read()).await);
        assert_bounded(started);
        assert_eq!(filesystem.close_attempts(), EXPECTED_CLOSE_ATTEMPTS);
        assert_sftp_reclaimed(&filesystem, NO_DROP_RECLAIMED_HANDLES).await;
    }
}
