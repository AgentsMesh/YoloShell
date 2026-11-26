const FIXTURE_HOME_COMPONENT: &str = "h";
const FIXTURE_RANDOM_BYTES: usize = 4;
const FIXTURE_ROOT_PREFIX: &str = "w";
const FIXTURE_ROOT_PARENT_REQUIRED: &str = "worker fixture socket root requires a parent";
const SOCKET_ROOT_REQUIRED: &str = "isolated worker tests require YOLO_SOCKET_ROOT";
#[cfg(target_os = "macos")]
const MACOS_PRIVATE_TEMP_ROOT: &str = "/private/tmp";
#[cfg(target_os = "macos")]
const MACOS_VISIBLE_TEMP_ROOT: &str = "/tmp";

struct FixtureFilesystem {
    paths: Paths,
    home_root: tempfile::TempDir,
}

impl FixtureFilesystem {
    fn new() -> Self {
        let socket_root = std::env::var_os(crate::paths::SOCKET_ROOT_ENVIRONMENT)
            .map(std::path::PathBuf::from)
            .expect(SOCKET_ROOT_REQUIRED);
        Self::new_with_socket_root(socket_root)
    }

    fn new_with_socket_root(socket_root: std::path::PathBuf) -> Self {
        let parent = socket_root.parent().expect(FIXTURE_ROOT_PARENT_REQUIRED);
        let root = tempfile::Builder::new()
            .prefix(FIXTURE_ROOT_PREFIX)
            .rand_bytes(FIXTURE_RANDOM_BYTES)
            .tempdir_in(parent)
            .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            root.path(),
            std::fs::Permissions::from_mode(crate::paths::test_support::private_directory_mode()),
        )
        .unwrap();
        let home = root.path().join(FIXTURE_HOME_COMPONENT);
        crate::paths::directory::PrivateDirectory::ensure(&home).unwrap();
        Self {
            paths: Paths::new_with_socket_root(home, visible_socket_root(&socket_root)),
            home_root: root,
        }
    }

    fn paths(&self) -> Paths {
        self.paths.clone()
    }

    fn scratch_path(&self, component: &str) -> std::path::PathBuf {
        self.home_root.path().join(component)
    }

    fn write_manifest(&self, record: &RuntimeRecord) {
        let manifest = crate::model::TerminalManifest {
            schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
            terminal_id: record.terminal_id,
            name: record.command.clone(),
            command: record.command.clone(),
            args: record.args.clone(),
            cwd: record.cwd.clone(),
            cols: record.cols,
            rows: record.rows,
            created_at_ms: record.started_at_ms,
            updated_at_ms: record.started_at_ms,
            archived: false,
            recovery_state: crate::model::RecoveryState::Created,
        };
        crate::paths::atomic_write_json(&self.paths.manifest(record.terminal_id), &manifest)
            .unwrap();
    }
}

fn write_launch_manifest(paths: &Paths, terminal_id: Uuid, launch: &WorkerLaunchSpec) {
    let timestamp = now_ms();
    let manifest = crate::model::TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: terminal_id,
        name: launch.command.clone(),
        command: launch.command.clone(),
        args: launch.args.clone(),
        cwd: launch.cwd.clone(),
        cols: launch.cols,
        rows: launch.rows,
        created_at_ms: timestamp,
        updated_at_ms: timestamp,
        archived: false,
        recovery_state: crate::model::RecoveryState::Created,
    };
    crate::paths::atomic_write_json(&paths.manifest(terminal_id), &manifest).unwrap();
}

#[cfg(target_os = "macos")]
fn visible_socket_root(physical: &std::path::Path) -> std::path::PathBuf {
    let relative = physical.strip_prefix(MACOS_PRIVATE_TEMP_ROOT).unwrap();
    std::path::Path::new(MACOS_VISIBLE_TEMP_ROOT).join(relative)
}

#[cfg(not(target_os = "macos"))]
fn visible_socket_root(physical: &std::path::Path) -> std::path::PathBuf {
    physical.to_path_buf()
}

#[cfg(all(feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/fixture_filesystem_tests.rs"]
mod tests;
