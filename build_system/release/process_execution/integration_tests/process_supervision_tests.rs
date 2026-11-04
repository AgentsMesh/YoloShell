use super::support::error_text;
use super::test_constants::{
    BASH_EXECUTABLE, LARGE_DUAL_OUTPUT_SCRIPT, LONG_SLEEP_SECONDS, OUTPUT_LIMIT_BYTES,
    PARENT_EXIT_SCRIPT, PRIVATE_DIRECTORY_MODE, PROCESS_CLEANUP_OBSERVATION_LIMIT_SECONDS,
    PROCESS_COMPLETION_LIMIT_SECONDS, SHELL_COMMAND_ARGUMENT, SLEEP_EXECUTABLE,
    SUCCESS_TIMEOUT_SECONDS, TERM_IGNORING_DESCENDANT_SCRIPT, TEST_LIST_ARGUMENT,
    TEST_TIMEOUT_MILLISECONDS,
};
use crate::process::ProcessCommand;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};
use yolo_release_filesystem::DirectoryHandle;

#[test]
fn retained_descriptor_cwd_survives_path_rename() {
    let fixture = Fixture::new();
    let original = fixture.workspace.display_path().to_owned();
    let moved = original.with_extension("moved");
    fs::rename(&original, &moved).unwrap();
    fs::create_dir(&original).unwrap();
    let output = fixture
        .command("/bin/pwd", Vec::new(), "retained cwd")
        .execute()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        fs::canonicalize(&moved).unwrap().display().to_string()
    );
    fs::remove_dir(&original).unwrap();
    fs::rename(&moved, &original).unwrap();
}

#[test]
fn parent_exit_cannot_leave_descendant_holding_pipes() {
    let fixture = Fixture::new();
    let started = Instant::now();
    let output = fixture
        .shell(
            PARENT_EXIT_SCRIPT,
            Duration::from_secs(SUCCESS_TIMEOUT_SECONDS),
        )
        .execute()
        .unwrap();
    assert!(output.status.success());
    assert!(started.elapsed() < Duration::from_secs(PROCESS_CLEANUP_OBSERVATION_LIMIT_SECONDS));
}

#[test]
fn term_ignoring_descendant_is_killed_before_deadline() {
    let fixture = Fixture::new();
    let started = Instant::now();
    let output = fixture
        .shell(
            TERM_IGNORING_DESCENDANT_SCRIPT,
            Duration::from_secs(PROCESS_COMPLETION_LIMIT_SECONDS),
        )
        .execute()
        .unwrap();
    assert!(output.status.success());
    assert!(started.elapsed() < Duration::from_secs(PROCESS_CLEANUP_OBSERVATION_LIMIT_SECONDS));
}

#[test]
fn large_stdout_and_stderr_fail_closed() {
    let fixture = Fixture::new();
    let error = error_text(
        fixture
            .shell(
                LARGE_DUAL_OUTPUT_SCRIPT,
                Duration::from_secs(PROCESS_COMPLETION_LIMIT_SECONDS),
            )
            .execute(),
    );
    assert!(error.contains("output exceeded the bounded limit"));
}

#[test]
fn command_specific_output_limit_accepts_a_bounded_gate_log() {
    let fixture = Fixture::new();
    let output = fixture
        .shell(
            &format!("printf '%*s' {} ''", OUTPUT_LIMIT_BYTES),
            Duration::from_secs(SUCCESS_TIMEOUT_SECONDS),
        )
        .with_output_limit(OUTPUT_LIMIT_BYTES)
        .execute()
        .unwrap();
    assert_eq!(output.stdout.len(), OUTPUT_LIMIT_BYTES);
}

#[test]
fn named_timeout_kills_entire_process_group() {
    let fixture = Fixture::new();
    let error = error_text(
        fixture
            .command(
                SLEEP_EXECUTABLE,
                vec![LONG_SLEEP_SECONDS.to_owned()],
                "named timeout child",
            )
            .with_timeout(Duration::from_millis(TEST_TIMEOUT_MILLISECONDS))
            .execute(),
    );
    assert!(error.contains("named timeout child timed out"));
}

#[test]
fn real_mach_o_runs_through_posix_spawn_supervisor() {
    let fixture = Fixture::new();
    let executable = std::env::current_exe().unwrap();
    let output = fixture
        .command(
            &executable,
            vec![TEST_LIST_ARGUMENT.to_owned()],
            "real Mach-O child",
        )
        .with_timeout(Duration::from_secs(PROCESS_COMPLETION_LIMIT_SECONDS))
        .execute()
        .unwrap();
    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
}

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    workspace: DirectoryHandle,
    home: DirectoryHandle,
    temporary: DirectoryHandle,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(
            root.path(),
            fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        for name in ["workspace", "home", "tmp"] {
            fs::create_dir(root.path().join(name)).unwrap();
        }
        let root_handle = DirectoryHandle::open(root.path()).unwrap();
        Self {
            workspace: DirectoryHandle::open_at(&root_handle, "workspace").unwrap(),
            home: DirectoryHandle::open_at(&root_handle, "home").unwrap(),
            temporary: DirectoryHandle::open_at(&root_handle, "tmp").unwrap(),
            _root: root,
        }
    }

    pub(super) fn command(
        &self,
        executable: impl AsRef<Path>,
        arguments: Vec<String>,
        description: &str,
    ) -> ProcessCommand {
        ProcessCommand::new(
            executable.as_ref(),
            arguments,
            &self.workspace,
            &self.home,
            &self.temporary,
            description,
        )
    }

    fn shell(&self, script: &str, timeout: Duration) -> ProcessCommand {
        self.command(
            BASH_EXECUTABLE,
            vec![SHELL_COMMAND_ARGUMENT.to_owned(), script.to_owned()],
            "adversarial shell child",
        )
        .with_timeout(timeout)
    }
}
