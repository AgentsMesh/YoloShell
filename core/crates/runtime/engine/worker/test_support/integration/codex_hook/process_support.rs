use crate::agent::{FencedDescendants, ObservedProcess, ProcessSource};
use std::sync::Arc;

const TEST_EXECUTABLE: &str = "/test/codex";
const TEST_PROCESS_GROUP: i32 = 1;
const PROCESS_SOURCE_MUST_NOT_BE_USED: &str = "test process source must not be queried";

pub(super) fn stub_process_source(
    process: crate::process::ProcessIdentity,
) -> Arc<dyn ProcessSource> {
    Arc::new(StubProcessSource { process })
}

pub(super) fn failing_process_source() -> Arc<dyn ProcessSource> {
    Arc::new(FailingProcessSource)
}

#[derive(Debug)]
struct StubProcessSource {
    process: crate::process::ProcessIdentity,
}

#[derive(Debug)]
struct FailingProcessSource;

impl ProcessSource for FailingProcessSource {
    fn descendants(&self, _root_pid: u32) -> anyhow::Result<Vec<ObservedProcess>> {
        anyhow::bail!(PROCESS_SOURCE_MUST_NOT_BE_USED)
    }

    fn fenced_descendants(
        &self,
        _root: crate::process::ProcessIdentity,
    ) -> anyhow::Result<FencedDescendants> {
        anyhow::bail!(PROCESS_SOURCE_MUST_NOT_BE_USED)
    }
}

impl ProcessSource for StubProcessSource {
    fn descendants(&self, _root_pid: u32) -> anyhow::Result<Vec<ObservedProcess>> {
        Ok(vec![observed(self.process)])
    }

    fn fenced_descendants(
        &self,
        _root: crate::process::ProcessIdentity,
    ) -> anyhow::Result<FencedDescendants> {
        Ok(FencedDescendants::Stable(vec![observed(self.process)]))
    }
}

fn observed(process: crate::process::ProcessIdentity) -> ObservedProcess {
    ObservedProcess {
        pid: process.pid,
        start_time: process.start_time,
        parent_pid: process.pid,
        process_group: TEST_PROCESS_GROUP,
        foreground_group: TEST_PROCESS_GROUP,
        executable: TEST_EXECUTABLE.to_owned(),
        argv: Vec::new(),
        environment: None,
        cwd: None,
        evidence_paths: None,
    }
}
