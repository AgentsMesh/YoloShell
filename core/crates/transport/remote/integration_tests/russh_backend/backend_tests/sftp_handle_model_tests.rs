use super::constants::*;
use super::memory_fs::{FailurePoint, MemoryFilesystem};
use super::sftp_read_support::*;
use crate::remote::MultiplexedSshConnection;
use crate::remote::constants::RUNTIME_UPLOAD_MODE;
use russh_sftp::protocol::{FileAttributes, OpenFlags};

#[test]
fn open_object_survives_same_path_regular_file_replacement() {
    let filesystem = MemoryFilesystem::default();
    let opened = filesystem
        .open(MEMORY_FILE, OpenFlags::READ, FileAttributes::empty())
        .unwrap();
    let original = filesystem.file_metadata(&opened).unwrap();
    filesystem.insert_file(MEMORY_FILE, REPLACEMENT_FILE_CONTENT, RUNTIME_UPLOAD_MODE);

    let packet_length = u32::try_from(SFTP_TEST_READ_LENGTH).unwrap();
    let data = filesystem
        .read_file(&opened, SFTP_TEST_READ_OFFSET, packet_length)
        .unwrap();
    let opened_after = filesystem.file_metadata(&opened).unwrap();
    let namespace_after = filesystem.metadata(MEMORY_FILE, false).unwrap();

    assert_eq!(data, &FILE_CONTENT[..SFTP_TEST_READ_LENGTH as usize]);
    assert_eq!(opened_after.size, original.size);
    assert_eq!(
        namespace_after.size,
        Some(REPLACEMENT_FILE_CONTENT.len() as u64)
    );
    assert_ne!(namespace_after.size, opened_after.size);
}

#[tokio::test]
async fn regular_file_replacement_after_open_is_denied_and_reclaimed() {
    let (fixture, filesystem) = fixture(normal_timeout()).await;
    filesystem.fail_once(FailurePoint::SftpReplaceAfterOpen);
    assert_denied(fixture.connection.read_file(&authorized_read()).await);
    assert_eq!(filesystem.close_attempts(), EXPECTED_CLOSE_ATTEMPTS);
    assert_sftp_reclaimed(&filesystem, NO_DROP_RECLAIMED_HANDLES).await;
}
