use super::*;
use constants::*;
use fixture::*;
use std::path::PathBuf;

#[tokio::test]
async fn in_memory_ssh_multiplexes_runtime_commands_and_sftp() {
    let fixture = connect_memory(true).await.unwrap();
    let known = trusted_hosts(fixture.connection.info());
    let runtime = RemoteRuntimeSpec::new(MEMORY_RUNTIME, MEMORY_HOME).unwrap();
    let mut host = RemoteHost::connect(fixture.connection.clone(), &known, runtime)
        .await
        .unwrap();
    let command_policy = CommandPolicy::new([NORMAL_PROGRAM.into()]).unwrap();
    let output = host
        .run_command(
            &command_policy,
            CommandRequest::new(NORMAL_PROGRAM, vec!["arg with spaces".into()]),
        )
        .await
        .unwrap();
    assert_eq!(output.stdout, COMMAND_STDOUT);
    assert_eq!(output.stderr, COMMAND_STDERR);
    assert_eq!(output.exit_status, Some(0));
    let policy = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    let file = host
        .read_file(&policy, FileReadRequest::new(MEMORY_FILE, 2, 6))
        .await
        .unwrap();
    assert_eq!(file.data, &FILE_CONTENT[2..8]);
    assert!(!file.eof);
    let tail = host
        .read_file(
            &policy,
            FileReadRequest::new(MEMORY_FILE, (FILE_CONTENT.len() - 2) as u64, 8),
        )
        .await
        .unwrap();
    assert_eq!(tail.data, &FILE_CONTENT[FILE_CONTENT.len() - 2..]);
    assert!(tail.eof);
    let attachment = host.open_attachment(uuid::Uuid::new_v4(), 0).await.unwrap();
    assert_eq!(attachment.initial.next_sequence, 1);
    attachment.detach().await.unwrap();
    let _: crate::model::HostStatus = host
        .request(crate::protocol::method::HOST_STATUS, &())
        .await
        .unwrap();
    fixture.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn real_command_channels_enforce_output_and_timeout_limits() {
    let fixture = connect_memory(false).await.unwrap();
    let large_policy = CommandPolicy::new([LARGE_PROGRAM.into()]).unwrap();
    let large = large_policy
        .authorize(
            CommandRequest::new(LARGE_PROGRAM, vec![])
                .with_limits(COMMAND_OUTPUT_TEST_TIMEOUT, COMMAND_CHANNEL_OUTPUT_LIMIT),
        )
        .unwrap();
    let output = fixture.connection.execute(&large).await.unwrap().value;
    assert_eq!(
        output.stdout.len() + output.stderr.len(),
        COMMAND_CHANNEL_OUTPUT_LIMIT
    );
    assert!(output.truncated);
    let hang_policy = CommandPolicy::new([HANG_PROGRAM.into()]).unwrap();
    let hanging = hang_policy
        .authorize(
            CommandRequest::new(HANG_PROGRAM, vec![])
                .with_limits(COMMAND_TIMEOUT, COMMAND_CHANNEL_OUTPUT_LIMIT),
        )
        .unwrap();
    let output = fixture.connection.execute(&hanging).await.unwrap().value;
    assert!(output.timed_out);
}

#[tokio::test]
async fn command_output_before_request_approval_is_preserved() {
    let fixture = connect_memory(false).await.unwrap();
    fixture
        .filesystem
        .fail_once(memory_fs::FailurePoint::ExecDataBeforeApproval);
    let policy = CommandPolicy::new([NORMAL_PROGRAM.into()]).unwrap();
    let output = fixture
        .connection
        .execute(
            &policy
                .authorize(CommandRequest::new(NORMAL_PROGRAM, vec![]))
                .unwrap(),
        )
        .await
        .unwrap()
        .value;
    assert_eq!(
        output.stdout,
        [APPROVAL_COMMAND_PREFIX, COMMAND_STDOUT].concat()
    );
    assert_eq!(output.stderr, COMMAND_STDERR);
    assert_eq!(output.exit_status, Some(0));
}

#[tokio::test]
async fn real_sftp_rechecks_resolved_path_on_read_channel() {
    let fixture = connect_memory(false).await.unwrap();
    let policy = FileReadPolicy::standard(vec![PathBuf::from("/srv/project")]).unwrap();
    let authorized = policy
        .authorize(FileReadRequest::new(MEMORY_LINK, 0, 4), MEMORY_LINK)
        .unwrap();
    assert!(matches!(
        fixture.connection.read_file(&authorized).await,
        Err(RemoteError::PathDenied(path)) if path.as_path() == std::path::Path::new(MEMORY_OUTSIDE)
    ));
}

#[tokio::test]
async fn ssh_agent_identity_authenticates_without_an_identity_file() {
    let state = tempfile::tempdir().unwrap();
    let (host_key, user_key) = keys();
    let known = known_for_key(&host_key);
    let agent_socket = agent::start(state.path(), &user_key).await;
    let (client_stream, server_stream) = tokio::io::duplex(MEMORY_STREAM_CAPACITY);
    spawn_server(server_stream, host_key, user_key.public_key().clone());
    let config = SshConnectConfig::new(endpoint(), known)
        .with_agent_socket(agent_socket)
        .with_timeouts(TEST_CONNECT_TIMEOUT, TEST_INACTIVITY_TIMEOUT);
    let connection = RusshConnection::connect_stream(config, client_stream)
        .await
        .unwrap();
    connection.disconnect().await.unwrap();
}
