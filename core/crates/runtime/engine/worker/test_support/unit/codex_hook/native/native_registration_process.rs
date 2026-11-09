use crate::agent::{FencedDescendants, ObservedProcess, ProcessSource};
use crate::process::ProcessIdentity;

const TEST_FOREGROUND_GROUP: i32 = 71_001;

pub(super) struct StableSource(pub(super) ProcessIdentity);

impl ProcessSource for StableSource {
    fn descendants(&self, root_pid: u32) -> anyhow::Result<Vec<ObservedProcess>> {
        let executable = std::env::current_exe()?
            .canonicalize()?
            .to_string_lossy()
            .into_owned();
        Ok(vec![ObservedProcess {
            pid: self.0.pid,
            start_time: self.0.start_time,
            parent_pid: root_pid,
            process_group: TEST_FOREGROUND_GROUP,
            foreground_group: TEST_FOREGROUND_GROUP,
            argv: vec![executable.clone()],
            executable,
            environment: None,
            cwd: None,
            evidence_paths: None,
        }])
    }

    fn fenced_descendants(&self, root: ProcessIdentity) -> anyhow::Result<FencedDescendants> {
        self.descendants(root.pid).map(FencedDescendants::Stable)
    }
}
