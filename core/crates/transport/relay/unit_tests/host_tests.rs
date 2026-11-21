use super::*;
use crate::model::{RecoveryState, RuntimeView, TerminalManifest, TerminalRuntimeStatus, TerminalView};

const TEST_WORKER_PID: u32 = 10;
const TEST_CHILD_PID: u32 = 11;
const TEST_HOME: &str = "/tmp";
const TEST_SOCKET: &str = "/tmp/yolo.sock";

fn view(status: Option<TerminalRuntimeStatus>, archived: bool) -> TerminalView {
    let terminal_id = Uuid::new_v4();
    TerminalView {
        manifest: TerminalManifest {
            schema_version: 1,
            terminal_id: terminal_id,
            name: "test".into(),
            command: "/bin/sh".into(),
            args: Vec::new(),
            cwd: PathBuf::from("/tmp"),
            cols: 80,
            rows: 24,
            created_at_ms: 1,
            updated_at_ms: 1,
            archived,
            recovery_state: RecoveryState::NeedsRecreate,
        },
        runtime: status.map(|status| RuntimeView {
            presentation: Default::default(),
            runtime_epoch: Uuid::new_v4(),
            worker_id: Uuid::new_v4(),
            worker_pid: TEST_WORKER_PID,
            worker_start_time: TEST_WORKER_PID as u64,
            child_pid: TEST_CHILD_PID,
            child_start_time: TEST_CHILD_PID as u64,
            status,
            last_output_sequence: 0,
            exit: None,
            bootstrap_delivery: crate::model::BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}

#[test]
fn session_plan_attaches_live_and_resumes_only_recoverable_sessions() {
    for status in [TerminalRuntimeStatus::Running, TerminalRuntimeStatus::Starting] {
        assert!(matches!(
            session_plan(&view(Some(status), false)),
            SessionPlan::Attach(PreparedSession::Active)
        ));
    }
    assert_eq!(
        session_plan(&view(Some(TerminalRuntimeStatus::Unreachable), false)),
        SessionPlan::Resume
    );
    for status in [
        TerminalRuntimeStatus::Exited,
        TerminalRuntimeStatus::Finalizing,
        TerminalRuntimeStatus::OutputBlocked,
    ] {
        assert_eq!(
            session_plan(&view(Some(status), false)),
            SessionPlan::PendingFinalization
        );
    }
    assert_eq!(session_plan(&view(None, false)), SessionPlan::Resume);
}

#[test]
fn session_plan_never_resumes_an_archived_session() {
    assert_eq!(
        session_plan(&view(Some(TerminalRuntimeStatus::Exited), true)),
        SessionPlan::Archived
    );
}

#[test]
fn runtime_roles_preserve_the_exact_image_even_when_its_filename_changes() {
    for executable in [
        "/Applications/Yolo.app/Contents/Helpers/yolo",
        "/private/test-runtime/yolo-adversarial",
        "/private/test-runtime/renamed-image",
    ] {
        assert_eq!(
            runtime_executable_path(Path::new(executable)).unwrap(),
            Path::new(executable)
        );
    }
    for invalid in ["", "relative-image", "/"] {
        assert!(matches!(
            runtime_executable_path(Path::new(invalid)),
            Err(RelayError::InvalidRuntimeExecutable(_))
        ));
    }

    let current = std::env::current_exe().unwrap();
    let managed = RelayRuntime::managed(TEST_HOME, TEST_SOCKET, None).unwrap();
    assert_eq!(managed.launch.unwrap().executable, current);
}
