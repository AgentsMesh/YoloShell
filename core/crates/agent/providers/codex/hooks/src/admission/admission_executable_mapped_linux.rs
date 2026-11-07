use super::{CodexExecutableAttestation, CodexExecutableKind, EXECUTABLE_CHANGED};
use anyhow::{Context, Result, ensure};
use std::fs::File;

const MAPPED_EXECUTABLE_CONTEXT: &str = "Codex hook mapped executable does not match admission";

impl CodexExecutableAttestation {
    pub fn verify_mapped_process(&self, pid: u32, kind: CodexExecutableKind) -> Result<()> {
        let (captured, named) = Self::capture_open(&self.path, kind)?;
        ensure!(captured == *self, EXECUTABLE_CHANGED);
        let mapped = crate::process::linux_native::open_executable(pid)
            .context(MAPPED_EXECUTABLE_CONTEXT)?;
        self.require_mapped_file(&mapped, kind)?;
        self.require_mapped_file(&named, kind)?;
        let (current, _) = Self::capture_open(&self.path, kind)?;
        ensure!(current == *self, EXECUTABLE_CHANGED);
        self.require_mapped_file(&mapped, kind)?;
        let still_mapped = crate::process::linux_native::open_executable(pid)
            .context(MAPPED_EXECUTABLE_CONTEXT)?;
        self.require_mapped_file(&still_mapped, kind)
    }

    fn require_mapped_file(&self, file: &File, kind: CodexExecutableKind) -> Result<()> {
        let identity = super::identity(&file.metadata().context(MAPPED_EXECUTABLE_CONTEXT)?, kind)?;
        ensure!(identity == self.identity, MAPPED_EXECUTABLE_CONTEXT);
        Ok(())
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/mapped_executable_linux_tests.rs"]
mod tests;
