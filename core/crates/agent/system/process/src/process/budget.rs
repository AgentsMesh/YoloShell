use super::super::constants::{
    MAX_FOREGROUND_PROCESS_IDS, MAX_PROCESS_ANCESTOR_STEPS, MAX_PROCESS_ARGV_BYTES,
    MAX_PROCESS_PATH_BYTES, MAX_SINGLE_PROCESS_ARGV_BYTES,
};
use anyhow::{Result, ensure};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) struct ScanBudget {
    remaining_pids: usize,
    remaining_ancestor_steps: usize,
    remaining_argv_bytes: usize,
    remaining_path_bytes: usize,
}

impl Default for ScanBudget {
    fn default() -> Self {
        Self::new(
            MAX_FOREGROUND_PROCESS_IDS,
            MAX_PROCESS_ANCESTOR_STEPS,
            MAX_PROCESS_ARGV_BYTES,
            MAX_PROCESS_PATH_BYTES,
        )
    }
}

impl ScanBudget {
    pub(super) fn new(pids: usize, ancestors: usize, argv: usize, paths: usize) -> Self {
        Self {
            remaining_pids: pids,
            remaining_ancestor_steps: ancestors,
            remaining_argv_bytes: argv,
            remaining_path_bytes: paths,
        }
    }

    pub(super) fn reserve_pids(&mut self, count: usize) -> Result<()> {
        ensure!(
            count <= self.remaining_pids,
            "foreground process id budget exceeded"
        );
        self.remaining_pids -= count;
        Ok(())
    }

    pub(super) fn record_ancestor_step(&mut self) -> Result<()> {
        ensure!(
            self.remaining_ancestor_steps > 0,
            "process ancestry budget exceeded"
        );
        self.remaining_ancestor_steps -= 1;
        Ok(())
    }

    pub(super) fn argv_limit(&self) -> usize {
        self.remaining_argv_bytes.min(MAX_SINGLE_PROCESS_ARGV_BYTES)
    }

    pub(super) fn record_argv(&mut self, bytes: usize) -> Result<()> {
        ensure!(bytes <= self.argv_limit(), "process argv budget exceeded");
        self.remaining_argv_bytes -= bytes;
        Ok(())
    }

    pub(super) fn record_path(&mut self, path: &Path) -> Result<()> {
        let bytes = path.as_os_str().as_bytes().len();
        ensure!(
            bytes <= self.remaining_path_bytes,
            "process path budget exceeded"
        );
        self.remaining_path_bytes -= bytes;
        Ok(())
    }
}
