#[test]
fn control_existing_host_mode_requires_home_and_rejects_socket_overrides() {
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
            cli::EXISTING_HOST_ARGUMENT,
            cli::STATUS_COMMAND,
        ])
        .is_ok()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::EXISTING_HOST_ARGUMENT,
            cli::STATUS_COMMAND,
        ])
        .is_err()
    );
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
            cli::SOCKET_ARGUMENT,
            "/tmp/host.sock",
            cli::EXISTING_HOST_ARGUMENT,
            cli::STATUS_COMMAND,
        ])
        .is_err()
    );
}

#[tokio::test]
async fn existing_host_mode_never_creates_or_starts_a_missing_home() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(MISSING_HOME_NAME);
    for command in [cli::STATUS_COMMAND, cli::LIST_COMMAND] {
        run_existing_host(&home, command).await.unwrap();
    }
    assert!(
        run_existing_host(&home, cli::SHUTDOWN_IF_IDLE_COMMAND)
            .await
            .is_err()
    );
    assert!(!home.exists());
}

#[tokio::test]
async fn idle_shutdown_command_never_starts_a_missing_managed_host() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(MISSING_HOME_NAME);
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        home.to_str().unwrap(),
        cli::SHUTDOWN_IF_IDLE_COMMAND,
    ])
    .unwrap();
    let Command::Ctl(options) = parsed.command else {
        panic!("{CONTROL_PARSE_ERROR}");
    };

    assert!(ctl::run(options).await.is_err());
    assert!(!home.exists());
}

#[tokio::test]
async fn unrestricted_existing_home_is_not_normalized_by_read_only_queries() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(EXISTING_HOME_NAME);
    std::fs::create_dir(&home).unwrap();
    std::fs::set_permissions(
        &home,
        std::fs::Permissions::from_mode(TEST_PUBLIC_DIRECTORY_MODE),
    )
    .unwrap();
    let socket_directory = Paths::new(&home).socket_dir().unwrap();
    for command in [
        cli::STATUS_COMMAND,
        cli::LIST_COMMAND,
        cli::SHUTDOWN_IF_IDLE_COMMAND,
    ] {
        assert!(run_existing_host(&home, command).await.is_err());
        assert!(!socket_directory.exists());
        assert_eq!(
            std::fs::metadata(&home).unwrap().permissions().mode()
                & yolo_runtime::paths::constants::PERMISSION_MODE_MASK,
            TEST_PUBLIC_DIRECTORY_MODE
        );
    }
}

#[tokio::test]
async fn private_existing_home_without_namespace_reports_absence_without_starting() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(EXISTING_HOME_NAME);
    yolo_runtime::paths::directory::PrivateDirectory::ensure(&home).unwrap();
    let socket_directory = Paths::new(&home).socket_dir().unwrap();
    for command in [cli::STATUS_COMMAND, cli::LIST_COMMAND] {
        run_existing_host(&home, command).await.unwrap();
        assert!(!socket_directory.exists());
        assert_eq!(std::fs::read_dir(&home).unwrap().count(), 0);
    }
}

#[tokio::test]
async fn corrupt_binding_is_never_repaired_or_auto_started() {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(CORRUPT_HOME_NAME);
    yolo_runtime::paths::directory::PrivateDirectory::ensure(&home).unwrap();
    let paths = Paths::new(&home);
    let socket_directory = paths.socket_dir().unwrap();
    std::fs::create_dir_all(&socket_directory).unwrap();
    std::fs::set_permissions(
        &socket_directory,
        std::fs::Permissions::from_mode(TEST_PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    let binding = socket_directory.join(BINDING_FILE_NAME);
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(TEST_PRIVATE_FILE_MODE)
        .open(&binding)
        .unwrap();
    file.write_all(CORRUPT_BINDING_BYTES).unwrap();
    file.sync_all().unwrap();
    for command in [
        cli::STATUS_COMMAND,
        cli::LIST_COMMAND,
        cli::SHUTDOWN_IF_IDLE_COMMAND,
    ] {
        assert!(run_existing_host(&home, command).await.is_err());
        assert_eq!(std::fs::read(&binding).unwrap(), CORRUPT_BINDING_BYTES);
        assert!(!socket_directory.join(HOST_SOCKET_FILE_NAME).exists());
    }
    std::fs::remove_file(binding).unwrap();
    std::fs::remove_dir(socket_directory).unwrap();
}

async fn run_existing_host(home: &std::path::Path, command: &str) -> anyhow::Result<()> {
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        home.to_str().unwrap(),
        cli::EXISTING_HOST_ARGUMENT,
        command,
    ])
    .unwrap();
    let Command::Ctl(options) = parsed.command else {
        panic!("{CONTROL_PARSE_ERROR}");
    };
    ctl::run(options).await
}

const MISSING_HOME_NAME: &str = "missing-home";
const EXISTING_HOME_NAME: &str = "existing-home";
const CORRUPT_HOME_NAME: &str = "corrupt-home";
const CONTROL_PARSE_ERROR: &str = "control command was not parsed";
const BINDING_FILE_NAME: &str = ".namespace.json";
const HOST_SOCKET_FILE_NAME: &str = "host.sock";
const CORRUPT_BINDING_BYTES: &[u8] = b"{corrupt";
const TEST_PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const TEST_PRIVATE_FILE_MODE: u32 = 0o600;
const TEST_PUBLIC_DIRECTORY_MODE: u32 = 0o755;
