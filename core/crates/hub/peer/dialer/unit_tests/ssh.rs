use super::constants::SOCKET;
use super::fixture::{deadline, identity};
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_peer_dialer::{PEER_SOCKET_FLAG, PEER_STDIO_FLAG, SystemSshDialer, dial_peer};

#[tokio::test]
async fn ssh_child_stdio_carries_one_peer_stream() {
    let directory = tempdir().unwrap();
    let program = directory.path().join("ssh");
    let args = directory.path().join("args");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nexec cat\n",
            args.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    let address = PeerAddress::ssh("test-target", SOCKET.into()).unwrap();
    let dialer = SystemSshDialer::new(&program);
    let mut io = dial_peer(&dialer, identity(), &address, deadline())
        .await
        .unwrap();
    io.write_all(b"hello").await.unwrap();
    let mut reply = [0; 5];
    io.read_exact(&mut reply).await.unwrap();
    assert_eq!(&reply, b"hello");
    drop(io);
    let recorded = std::fs::read_to_string(&args).unwrap();
    assert!(recorded.contains(PEER_STDIO_FLAG));
    assert!(recorded.contains(PEER_SOCKET_FLAG));
    assert!(recorded.contains(SOCKET));
}

#[tokio::test]
async fn a_managed_profile_dial_is_not_accepted_by_the_ssh_dialer() {
    let address = PeerAddress::profile("managed", SOCKET.into()).unwrap();
    assert!(
        dial_peer(
            &SystemSshDialer::new("ssh"),
            identity(),
            &address,
            deadline()
        )
        .await
        .is_err()
    );
}
