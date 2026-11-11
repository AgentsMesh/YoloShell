use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn home_replacement_blocks_a_second_host_and_fences_the_first() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let displaced = root.path().join("displaced-home");
    let first = Host::open(HostConfig::new(&home)).unwrap();
    std::fs::rename(&home, &displaced).unwrap();
    Paths::new(&home).ensure_base().unwrap();

    let busy = Host::open(HostConfig::new(&home)).err().unwrap();
    assert!(busy.to_string().contains("already owns the home"));
    let response = first
        .dispatch(&request(method::HOST_STATUS, &serde_json::Value::Null))
        .await;
    assert!(!response.ok);
    assert!(first.namespace.verify().is_err());

    drop(first);
    assert!(Host::open(HostConfig::new(&home)).is_ok());
}

#[tokio::test]
async fn socket_parent_replacement_retains_the_external_endpoint_lock() {
    let root = tempdir().unwrap();
    let socket_parent = root.path().join("socket-parent");
    let displaced = root.path().join("displaced-socket-parent");
    std::fs::create_dir(&socket_parent).unwrap();
    let socket = socket_parent.join("host.sock");
    let mut first_config = HostConfig::new(root.path().join("home-one"));
    first_config.socket = Some(socket.clone());
    let first = Host::open(first_config).unwrap();
    std::fs::rename(&socket_parent, displaced).unwrap();
    std::fs::create_dir(&socket_parent).unwrap();

    let mut second_config = HostConfig::new(root.path().join("home-two"));
    second_config.socket = Some(socket);
    assert!(Host::open(second_config.clone()).is_err());
    let response = first
        .dispatch(&request(method::HOST_STATUS, &serde_json::Value::Null))
        .await;
    assert!(!response.ok);

    drop(first);
    assert!(Host::open(second_config).is_ok());
}

#[tokio::test]
async fn sessions_replacement_permanently_fences_the_host() {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let host = Host::open(HostConfig::new(&home)).unwrap();
    let sessions = host.paths.sessions_dir();
    let displaced = root.path().join("displaced-sessions");
    std::fs::rename(&sessions, &displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&sessions).unwrap();

    let response = host
        .dispatch(&request(method::HOST_STATUS, &serde_json::Value::Null))
        .await;

    assert!(!response.ok);
    assert!(host.namespace.verify().is_err());
    assert!(displaced.is_dir());
}
