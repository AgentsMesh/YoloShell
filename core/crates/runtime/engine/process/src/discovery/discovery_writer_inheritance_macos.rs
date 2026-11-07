use super::{
    INHERITANCE_COMMITTED_DETAIL, INHERITANCE_OWNER_DETAIL, ProcessDiscoveryCapability,
    ProcessDiscoveryWriterInheritance, lock_writer,
};
use crate::process::WaitableChildSpawnBoundary;
use crate::process::{PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV, PROCESS_DISCOVERY_CAPABILITY_ENV};
use anyhow::{Result, ensure};
use std::process::Command;
use std::sync::Arc;

impl ProcessDiscoveryWriterInheritance {
    pub(crate) fn spawn_boundary(&self) -> WaitableChildSpawnBoundary<'_> {
        self.spawn_guard
            .as_ref()
            .expect(INHERITANCE_COMMITTED_DETAIL)
            .boundary()
    }

    pub fn descriptor(&self) -> libc::c_int {
        lock_writer(&self.platform.writer)
            .descriptor
            .as_ref()
            .expect(INHERITANCE_COMMITTED_DETAIL)
            .descriptor()
    }

    pub fn configure_child(
        &self,
        capability: &ProcessDiscoveryCapability,
        command: &mut Command,
    ) -> Result<()> {
        ensure!(
            Arc::ptr_eq(&self.platform.writer, &capability.platform.writer),
            INHERITANCE_OWNER_DETAIL
        );
        command.env(PROCESS_DISCOVERY_CAPABILITY_ENV, capability.value());
        command.env(
            PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV,
            self.descriptor().to_string(),
        );
        lock_writer(&self.platform.writer)
            .descriptor
            .as_ref()
            .expect(INHERITANCE_COMMITTED_DETAIL)
            .configure_exec_inheritance(command);
        Ok(())
    }

    pub(super) fn finish(&mut self) -> Result<()> {
        let mut state = lock_writer(&self.platform.writer);
        ensure!(state.active, INHERITANCE_COMMITTED_DETAIL);
        let writer = state
            .descriptor
            .take()
            .ok_or_else(|| anyhow::anyhow!(INHERITANCE_COMMITTED_DETAIL))?;
        state.active = false;
        drop(state);
        drop(writer);
        self.finished = true;
        self.spawn_guard.take();
        Ok(())
    }
}

impl Drop for ProcessDiscoveryWriterInheritance {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.finish();
        }
    }
}
