use super::constants::MEMORY_RUNTIME_PARENT;
use super::fixture::connect_memory;
use crate::remote::ChannelPurpose;
use russh_sftp::client::error::Error as SftpError;
use russh_sftp::protocol::{FileAttributes, StatusCode};

#[tokio::test]
async fn server_rejects_default_sftp_attribute_mutations() {
    let fixture = connect_memory(false).await.unwrap();
    let (_, sftp) = fixture
        .connection
        .open_sftp(ChannelPurpose::RuntimeDeployment)
        .await
        .unwrap();

    let error = sftp
        .set_metadata(MEMORY_RUNTIME_PARENT, FileAttributes::default())
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        SftpError::Status(status) if status.status_code == StatusCode::PermissionDenied
    ));
    sftp.close().await.unwrap();
}
