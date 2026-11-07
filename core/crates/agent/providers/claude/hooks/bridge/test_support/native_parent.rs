use crate::native_parent::Parent;
use std::collections::BTreeMap;

pub const HELPER_PID: u32 = 101;
pub const SHELL_PID: u32 = 102;
pub const AGENT_PID: u32 = 103;
pub const START_TIME: u64 = 10_001;
pub const OTHER_START: u64 = 10_002;
pub const HELPER: &str = "/opt/yolo/yolo-host";
pub const AGENT: &str = "/opt/claude/claude";
pub const MISSING_PARENT: &str = "missing fixture parent";

pub fn process(pid: u32, parent_pid: u32, executable: &str) -> Parent {
    Parent {
        pid,
        parent_pid,
        start_time: START_TIME,
        executable: executable.into(),
    }
}

pub fn lineage(shell: &str) -> BTreeMap<u32, Parent> {
    BTreeMap::from([
        (HELPER_PID, process(HELPER_PID, SHELL_PID, HELPER)),
        (SHELL_PID, process(SHELL_PID, AGENT_PID, shell)),
        (AGENT_PID, process(AGENT_PID, u32::default(), AGENT)),
    ])
}
