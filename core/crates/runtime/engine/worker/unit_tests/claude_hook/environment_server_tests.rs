use super::{native_environment::ClaudeHookEnvironment, server::ClaudeHookServer};
use crate::paths::{IsolatedTestSocketRoot, Paths};
use crate::worker::WorkerNamespace;
use portable_pty::CommandBuilder;
use std::ffi::OsStr;
use uuid::Uuid;

const INHERITED_PATH: &str = "/usr/bin:/bin";
const TEST_COMMAND: &str = "/bin/sh";
const TEST_HOST_SOURCE: &str = "/bin/echo";
const MALFORMED_ARTIFACT_PREFIX: &str = "malformed-bind-artifact";
const MALFORMED_ARTIFACT_CONTENTS: &[u8] = b"untrusted residue";
const MALFORMED_ARTIFACT_COUNT: usize = crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES + 1;

#[test]
fn native_environment_preserves_path_and_shell_configuration() {
    let root = tempfile::tempdir().unwrap();
    let mut socket_root = IsolatedTestSocketRoot::create().unwrap();
    let paths =
        Paths::new_with_socket_root(root.path().join("home"), socket_root.path().to_owned());
    let terminal_id = Uuid::new_v4();
    super::test_session_manifest::ensure(&paths, terminal_id);
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let environment = ClaudeHookEnvironment::new(
        &namespace,
        &paths,
        terminal_id,
        std::env::current_exe().unwrap().canonicalize().unwrap(),
    )
    .unwrap();
    let mut command = CommandBuilder::new(TEST_COMMAND);
    command.env(crate::agent::constants::PATH_ENV, INHERITED_PATH);
    let arguments = command.get_argv().to_owned();
    environment.configure(&mut command).unwrap();
    assert_eq!(
        command.get_env(crate::claude_hooks::CLAUDE_HOOK_SOCKET_ENV),
        Some(environment.socket_path().as_os_str())
    );
    assert_eq!(
        command.get_env(crate::claude_hooks::CLAUDE_HOOK_TOKEN_ENV),
        None
    );
    assert_eq!(
        command.get_env(crate::claude_hooks::CLAUDE_SETTINGS_DIRECTORY_ENV),
        None
    );
    assert_eq!(
        command.get_env(crate::agent::constants::PATH_ENV),
        Some(OsStr::new(INHERITED_PATH))
    );
    assert_eq!(command.get_argv(), &arguments);
    drop((environment, namespace));
    socket_root.finalize().unwrap();
}

#[tokio::test]
async fn listener_restart_preserves_the_shell_issued_endpoint_and_native_bridge() {
    let root = tempfile::tempdir().unwrap();
    let mut socket_root = IsolatedTestSocketRoot::create().unwrap();
    let paths =
        Paths::new_with_socket_root(root.path().join("home"), socket_root.path().to_path_buf());
    let terminal_id = Uuid::new_v4();
    crate::worker::claude_hook::test_session_manifest::ensure(&paths, terminal_id);
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let settings = paths
        .session_dir(terminal_id)
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    let mut server = ClaudeHookServer::bind_test_source(
        &namespace,
        &paths,
        terminal_id,
        settings,
        std::path::Path::new(TEST_HOST_SOURCE),
    )
    .await
    .unwrap();
    let path = server.environment().socket_path().to_path_buf();
    let hook = server.environment().executable().path().to_owned();
    assert_eq!(
        server.environment().executable().path(),
        std::path::Path::new(TEST_HOST_SOURCE)
            .canonicalize()
            .unwrap()
    );

    server.restart(&namespace).await.unwrap();

    assert_eq!(server.environment().socket_path(), path);
    assert_eq!(server.environment().executable().path(), hook);
    let client = tokio::net::UnixStream::connect(path).await.unwrap();
    let (accepted, _) = server.accept().await.unwrap();
    assert_eq!(super::peer::pid(&accepted).unwrap(), std::process::id());
    drop((client, accepted));
    drop(server);
    drop(namespace);
    socket_root.finalize().unwrap();
}

#[tokio::test]
async fn malformed_scan_overflow_cannot_prevent_hook_server_bind() {
    let root = tempfile::tempdir().unwrap();
    let mut socket_root = IsolatedTestSocketRoot::create().unwrap();
    let paths =
        Paths::new_with_socket_root(root.path().join("home"), socket_root.path().to_path_buf());
    let terminal_id = Uuid::new_v4();
    crate::worker::claude_hook::test_session_manifest::ensure(&paths, terminal_id);
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let settings = paths
        .session_dir(terminal_id)
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    namespace.ensure_directory(&settings).unwrap();
    for ordinal in 0..MALFORMED_ARTIFACT_COUNT {
        let path = settings.join(format!("{MALFORMED_ARTIFACT_PREFIX}-{ordinal}"));
        crate::paths::private_file::PrivateFileEntry::open(&path)
            .unwrap()
            .create_bytes_exclusive(
                MALFORMED_ARTIFACT_CONTENTS,
                crate::claude_hooks::MAX_CLAUDE_SETTINGS_BYTES,
            )
            .unwrap();
    }

    let server = ClaudeHookServer::bind_test_source(
        &namespace,
        &paths,
        terminal_id,
        settings,
        std::path::Path::new(TEST_HOST_SOURCE),
    )
    .await
    .unwrap();

    assert!(server.environment().socket_path().exists());
    drop(server);
    drop(namespace);
    socket_root.finalize().unwrap();
}
