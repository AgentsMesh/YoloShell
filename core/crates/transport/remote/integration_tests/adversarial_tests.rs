use super::super::*;
use super::support::{connection, known_hosts};
use std::path::PathBuf;

const REMOTE_HOME: &str = "/var/lib/yolo";

fn runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::new("/opt/yolo/yolo", REMOTE_HOME).unwrap()
}

#[tokio::test]
async fn confused_channel_is_rejected() {
    let connection = connection(REMOTE_HOME, "boot-a");
    connection.set_purpose_override(ChannelPurpose::Command);
    let error = RemoteHost::connect(connection, &known_hosts(), runtime())
        .await
        .err()
        .unwrap();
    assert!(matches!(error, RemoteError::ChannelMismatch));
}

#[tokio::test]
async fn preflight_denial_does_not_open_a_file_channel() {
    let connection = connection(REMOTE_HOME, "boot-a");
    let host = RemoteHost::connect(connection.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    let policy = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    let denied = host
        .read_file(
            &policy,
            FileReadRequest::new("/srv/project/../secret", 0, 4),
        )
        .await;
    assert!(matches!(denied, Err(RemoteError::PathDenied(_))));
    assert_eq!(connection.purposes(), [ChannelPurpose::RuntimeControl]);
}

#[tokio::test]
async fn backend_cannot_return_a_different_or_oversized_file() {
    let connection = connection(REMOTE_HOME, "boot-a");
    connection.set_file_result(FileReadResult {
        resolved_path: PathBuf::from("/srv/project/other"),
        data: vec![0; 16],
        eof: false,
    });
    let host = RemoteHost::connect(connection, &known_hosts(), runtime())
        .await
        .unwrap();
    let policy = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    let result = host
        .read_file(&policy, FileReadRequest::new("/srv/project/file", 0, 4))
        .await;
    assert!(matches!(result, Err(RemoteError::LimitExceeded)));
}

#[tokio::test]
async fn disconnected_host_rejects_new_side_channels() {
    let connection = connection(REMOTE_HOME, "boot-a");
    let mut host = RemoteHost::connect(connection.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    host.mark_transport_lost();
    let commands = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    assert!(matches!(
        host.run_command(&commands, CommandRequest::new("/usr/bin/git", vec![]))
            .await,
        Err(RemoteError::Disconnected)
    ));
    assert!(matches!(
        host.open_attachment(uuid::Uuid::new_v4(), 0).await,
        Err(RemoteError::Disconnected)
    ));
    let files = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    assert!(matches!(
        host.read_file(&files, FileReadRequest::new("/srv/project/file", 0, 1))
            .await,
        Err(RemoteError::Disconnected)
    ));
    assert_eq!(connection.purposes(), [ChannelPurpose::RuntimeControl]);
}

#[tokio::test]
async fn attachment_channel_purpose_is_rechecked() {
    let connection = connection(REMOTE_HOME, "boot-a");
    let host = RemoteHost::connect(connection.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    connection.set_purpose_override(ChannelPurpose::Command);
    assert!(matches!(
        host.open_attachment(uuid::Uuid::new_v4(), 0).await,
        Err(RemoteError::ChannelMismatch)
    ));
}
