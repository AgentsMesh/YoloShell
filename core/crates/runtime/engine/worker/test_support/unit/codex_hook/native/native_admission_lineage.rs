use super::{ObservedProcess, ProcessIdentity};
use std::path::Path;

pub(super) const SHELL_PID: u32 = 61_001;
pub(super) const AGENT_PID: u32 = 61_002;
pub(super) const WRAPPER_PID: u32 = 61_003;
pub(super) const CALLBACK_PID: u32 = 61_004;
pub(super) const START_TIME: u64 = 81_001;
pub(super) const FOREGROUND_GROUP: i32 = 71_001;
pub(super) const IDENTITY_INCREMENT: u64 = 1;
pub(super) const GROUP_INCREMENT: i32 = 1;
pub(super) const AGENT_EXECUTABLE: &str = "/opt/codex/bin/codex";
pub(super) const HOOK_EXECUTABLE: &str = "/Applications/Yolo.app/Contents/MacOS/yolo-host";
pub(super) const COMMAND_SHELL: &str = "/bin/sh";
pub(super) const FOREIGN_EXECUTABLE: &str = "/opt/foreign/helper";

pub(super) fn identity(pid: u32) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        start_time: START_TIME,
    }
}

pub(super) fn process(pid: u32, parent_pid: u32, executable: &str) -> ObservedProcess {
    ObservedProcess {
        pid,
        parent_pid,
        start_time: START_TIME,
        process_group: FOREGROUND_GROUP,
        foreground_group: FOREGROUND_GROUP,
        executable: executable.to_owned(),
        argv: vec![executable.to_owned()],
        environment: None,
        cwd: None,
        evidence_paths: None,
    }
}

pub(super) fn direct() -> Vec<ObservedProcess> {
    vec![
        process(AGENT_PID, SHELL_PID, AGENT_EXECUTABLE),
        process(CALLBACK_PID, AGENT_PID, HOOK_EXECUTABLE),
    ]
}

pub(super) fn capture(
    processes: &[ObservedProcess],
) -> anyhow::Result<(ObservedProcess, Vec<ObservedProcess>)> {
    super::capture(
        identity(CALLBACK_PID),
        identity(SHELL_PID),
        Path::new(HOOK_EXECUTABLE),
        processes,
    )
}
