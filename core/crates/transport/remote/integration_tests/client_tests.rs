use super::super::constants::TEST_STATUS_ARGUMENT;
use super::super::*;
use super::support::{
    TEST_ATTACHMENT_AFTER_SEQUENCE, TEST_ATTACHMENT_NEXT_SEQUENCE, connection, known_hosts,
};
use crate::model::HostStatus;
use crate::protocol::{event, method};
use std::path::PathBuf;

const REMOTE_HOME: &str = "/var/lib/yolo";
const REMOTE_RUNTIME: &str = "/opt/yolo/yolo";

fn runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::new(REMOTE_RUNTIME, REMOTE_HOME).unwrap()
}

#[tokio::test]
async fn control_loss_and_reconnect_endpoint_drift_fail_closed() {
    let first = connection(REMOTE_HOME, "boot-a");
    first.break_control_after_handshake();
    let mut host = RemoteHost::connect(first.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    assert_eq!(host.status().boot_id, "boot-a");
    assert!(matches!(
        host.request::<_, HostStatus>(method::HOST_STATUS, &())
            .await,
        Err(RemoteError::Protocol(
            crate::protocol::ProtocolError::OutcomeUnknown
        ))
    ));
    assert!(matches!(host.state(), RemoteState::Disconnected { .. }));
    assert_eq!(first.purposes(), [ChannelPurpose::RuntimeControl]);

    let other = super::support::FakeConnection::new(
        RemoteEndpoint::standard("stone", "other.example").unwrap(),
        HostKey::new("ssh-ed25519", "encoded-key").unwrap(),
        PathBuf::from(REMOTE_HOME),
        "boot-b",
    );
    assert!(matches!(
        host.reconnect(other, &known_hosts()).await,
        Err(RemoteError::InvalidEndpoint(_))
    ));
}

#[tokio::test]
async fn ready_reconnect_and_side_channel_identity_checks_cover_each_path() {
    let first = connection(REMOTE_HOME, "boot-a");
    let mut host = RemoteHost::connect(first, &known_hosts(), runtime())
        .await
        .unwrap();
    let replacement = connection(REMOTE_HOME, "boot-b");
    let report = host.reconnect(replacement, &known_hosts()).await.unwrap();
    assert!(!report.same_boot);

    let command_policy = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    let connection = connection(REMOTE_HOME, "boot-a");
    let host = RemoteHost::connect(connection.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    connection.set_purpose_override(ChannelPurpose::FileRead);
    assert!(matches!(
        host.run_command(&command_policy, CommandRequest::new("/usr/bin/git", vec![]))
            .await,
        Err(RemoteError::ChannelMismatch)
    ));
    let policy = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    connection.set_purpose_override(ChannelPurpose::Command);
    assert!(matches!(
        host.read_file(&policy, FileReadRequest::new("/srv/project/file", 0, 4))
            .await,
        Err(RemoteError::ChannelMismatch)
    ));
    connection.set_purpose_overrides([ChannelPurpose::FileRead, ChannelPurpose::Command]);
    assert!(matches!(
        host.read_file(&policy, FileReadRequest::new("/srv/project/file", 0, 4))
            .await,
        Err(RemoteError::ChannelMismatch)
    ));
}

#[tokio::test]
async fn one_transport_opens_independent_control_attachment_command_and_file_channels() {
    let connection = connection(REMOTE_HOME, "boot-a");
    let mut host = RemoteHost::connect(connection.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    assert!(matches!(host.state(), RemoteState::Ready(_)));
    let mut attachment = host
        .open_attachment(uuid::Uuid::new_v4(), TEST_ATTACHMENT_AFTER_SEQUENCE)
        .await
        .unwrap();
    assert_eq!(
        attachment.initial.next_sequence,
        TEST_ATTACHMENT_NEXT_SEQUENCE
    );
    assert_eq!(
        attachment.next_event().await.unwrap().event,
        event::TERMINAL_OUTPUT
    );
    let before_detach = connection.requests();
    attachment.detach().await.unwrap();
    assert_eq!(connection.requests(), before_detach);

    let command_policy = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    let output = host
        .run_command(
            &command_policy,
            CommandRequest::new("/usr/bin/git", vec![TEST_STATUS_ARGUMENT.into()]),
        )
        .await
        .unwrap();
    assert_eq!(output.stdout, b"ok");

    let file_policy = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    let file = host
        .read_file(
            &file_policy,
            FileReadRequest::new("/srv/project/file", 0, 4),
        )
        .await
        .unwrap();
    assert_eq!(file.data, b"data");
    assert_eq!(
        connection.purposes(),
        [
            ChannelPurpose::RuntimeControl,
            ChannelPurpose::TerminalAttachment,
            ChannelPurpose::Command,
            ChannelPurpose::FileRead,
            ChannelPurpose::FileRead,
        ]
    );

    let _: HostStatus = host.request(method::HOST_STATUS, &()).await.unwrap();
}

#[tokio::test]
async fn reconnect_replaces_transport_without_terminating_remote_runtime() {
    let first = connection(REMOTE_HOME, "boot-a");
    let mut host = RemoteHost::connect(first.clone(), &known_hosts(), runtime())
        .await
        .unwrap();
    host.mark_transport_lost();
    assert!(matches!(
        host.request::<_, HostStatus>(crate::protocol::method::HOST_STATUS, &())
            .await,
        Err(RemoteError::Disconnected)
    ));
    let second = connection(REMOTE_HOME, "boot-a");
    let report = host
        .reconnect(second.clone(), &known_hosts())
        .await
        .unwrap();
    assert!(report.same_boot);
    assert_ne!(report.previous.connection_id, report.current.connection_id);
    assert_eq!(first.purposes(), [ChannelPurpose::RuntimeControl]);
    assert_eq!(second.purposes(), [ChannelPurpose::RuntimeControl]);
}
