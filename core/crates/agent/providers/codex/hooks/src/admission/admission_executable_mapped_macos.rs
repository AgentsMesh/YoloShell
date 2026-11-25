use super::{CodexExecutableAttestation, CodexExecutableKind, EXECUTABLE_CHANGED};
use anyhow::{Context, Result, ensure};
use std::os::fd::AsRawFd;

const MAPPED_EXECUTABLE_CONTEXT: &str = "Codex hook mapped executable does not match admission";

impl CodexExecutableAttestation {
    pub fn verify_mapped_process(&self, pid: u32, kind: CodexExecutableKind) -> Result<()> {
        let (captured, executable) = Self::capture_open(&self.path, kind)?;
        ensure!(captured == *self, EXECUTABLE_CHANGED);
        verify(pid, executable.as_raw_fd())?;
        let (current, _) = Self::capture_open(&self.path, kind)?;
        ensure!(current == *self, EXECUTABLE_CHANGED);
        verify(pid, executable.as_raw_fd())
    }
}

fn verify(pid: u32, descriptor: std::os::fd::RawFd) -> Result<()> {
    crate::process::macos_mapped_executable::verify_mapped_executable(pid, descriptor)
        .context(MAPPED_EXECUTABLE_CONTEXT)
        .map(drop)
}
