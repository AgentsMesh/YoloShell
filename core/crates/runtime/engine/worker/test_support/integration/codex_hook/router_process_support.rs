use crate::agent::{ObservedProcess, ProcessSource};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const TEST_EXECUTABLE: &str = "/test/codex";
const EXECUTABLE_CONTENT: &[u8] = b"#!/bin/sh\nexit 0\n";
const TEST_PROCESS_GROUP: i32 = 1;

pub(super) struct StableProcessSource {
    pub(super) process: crate::process::ProcessIdentity,
}

impl ProcessSource for StableProcessSource {
    fn descendants(&self, _root_pid: u32) -> anyhow::Result<Vec<ObservedProcess>> {
        Ok(vec![ObservedProcess {
            pid: self.process.pid,
            start_time: self.process.start_time,
            parent_pid: self.process.pid,
            process_group: TEST_PROCESS_GROUP,
            foreground_group: TEST_PROCESS_GROUP,
            executable: TEST_EXECUTABLE.to_owned(),
            argv: Vec::new(),
            environment: None,
            cwd: None,
            evidence_paths: None,
        }])
    }
}

pub(super) fn directory(path: &Path) -> PathBuf {
    std::fs::create_dir(path).unwrap();
    path.to_path_buf()
}

pub(super) fn executable(path: &Path) -> PathBuf {
    std::fs::write(path, EXECUTABLE_CONTENT).unwrap();
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(crate::constants::RUNTIME_EXECUTABLE_MODE),
    )
    .unwrap();
    path.to_path_buf()
}
