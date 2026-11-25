use super::ProcessDiscoveryCapability;
use crate::process::constants::PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV;
use crate::process::{
    CapturedProcessTree, LiveProcessIdentity, MacosProcessCapabilityChildWriter,
    MacosProcessCapabilityIssuer, MacosProcessCapabilityManagerReader,
    PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV, ProcessIdentity,
};
use anyhow::{Result, ensure};
use std::process::Command;
use std::sync::{Arc, Mutex};

#[path = "discovery_inherited_writer_macos.rs"]
mod inherited;
#[path = "discovery_writer_inheritance_macos.rs"]
mod writer_inheritance;

const INHERITANCE_ACTIVE_DETAIL: &str = "process discovery writer inheritance is already active";
const INHERITANCE_COMMITTED_DETAIL: &str = "process discovery writer inheritance is unavailable";
const INHERITANCE_OWNER_DETAIL: &str = "process discovery writer inheritance owner mismatch";

#[derive(Clone)]
pub(super) struct MacosDiscoveryCapability {
    authority: Arc<MacosProcessCapabilityManagerReader>,
    writer: Arc<Mutex<WriterState>>,
}

struct WriterState {
    descriptor: Option<MacosProcessCapabilityChildWriter>,
    active: bool,
}

pub struct ProcessDiscoveryWriterInheritance {
    platform: MacosDiscoveryCapability,
    spawn_guard: Option<crate::process::sigchld_waitability::WaitableChildSpawnGuard>,
    finished: bool,
}

pub struct ProcessDiscoveryInheritedWriter {
    value: String,
    writer: MacosProcessCapabilityChildWriter,
}

impl MacosDiscoveryCapability {
    pub(super) fn generate() -> Result<Self> {
        let issuer = MacosProcessCapabilityIssuer::create()?;
        let (authority, writer) = issuer.into_staged()?;
        Ok(Self {
            authority,
            writer: Arc::new(Mutex::new(WriterState {
                descriptor: Some(writer),
                active: false,
            })),
        })
    }

    fn adopted(authority: Arc<MacosProcessCapabilityManagerReader>) -> Self {
        Self {
            authority,
            writer: Arc::new(Mutex::new(WriterState {
                descriptor: None,
                active: false,
            })),
        }
    }
}

impl ProcessDiscoveryCapability {
    pub fn configure_manager_inheritance(&self, command: &mut Command) {
        command.env(PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV, &self.value);
        command.env(
            PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV,
            self.platform.authority.descriptor().to_string(),
        );
        self.platform.authority.configure_exec_inheritance(command);
    }

    pub fn adopt_inherited_manager() -> Result<Self> {
        let value = inherited::manager_value()?;
        let descriptor = inherited::descriptor()?;
        let authority = MacosProcessCapabilityManagerReader::from_transferred(descriptor)?;
        Ok(Self {
            value,
            platform: MacosDiscoveryCapability::adopted(authority),
        })
    }

    pub fn begin_writer_inheritance(&self) -> Result<ProcessDiscoveryWriterInheritance> {
        let spawn_guard = crate::process::waitable_child_spawn_guard()?;
        let mut state = lock_writer(&self.platform.writer);
        ensure!(!state.active, INHERITANCE_ACTIVE_DETAIL);
        ensure!(state.descriptor.is_some(), INHERITANCE_COMMITTED_DETAIL);
        state.active = true;
        drop(state);
        Ok(ProcessDiscoveryWriterInheritance {
            platform: self.platform.clone(),
            spawn_guard: Some(spawn_guard),
            finished: false,
        })
    }

    pub fn commit_writer_inheritance(
        &self,
        mut inheritance: ProcessDiscoveryWriterInheritance,
        identity: ProcessIdentity,
    ) -> Result<()> {
        ensure!(
            Arc::ptr_eq(&self.platform.writer, &inheritance.platform.writer),
            INHERITANCE_OWNER_DETAIL
        );
        let child = LiveProcessIdentity::capture(identity)?
            .ok_or_else(|| anyhow::anyhow!(INHERITANCE_COMMITTED_DETAIL))?;
        self.platform.authority.verify_writer_holder(&child)?;
        inheritance.finish()?;
        Ok(())
    }

    pub fn writer_handle(&self) -> u64 {
        self.platform.authority.writer_handle()
    }

    pub fn reader_handle(&self) -> u64 {
        self.platform.authority.identity().reader_handle()
    }

    pub fn verify_writer_holder(&self, identity: ProcessIdentity) -> Result<()> {
        let live = LiveProcessIdentity::capture(identity)?
            .ok_or_else(|| anyhow::anyhow!(INHERITANCE_COMMITTED_DETAIL))?;
        self.platform.authority.verify_writer_holder(&live)
    }

    pub fn capture_process_tree(&self, identity: ProcessIdentity) -> Result<CapturedProcessTree> {
        let mut tree =
            CapturedProcessTree::capture_with_discovery_internal(identity, self.value.clone())?;
        tree.install_process_capability(Arc::clone(&self.platform.authority))?;
        tree.refresh()?;
        Ok(tree)
    }

    pub(crate) fn descriptor_authority(&self) -> Arc<MacosProcessCapabilityManagerReader> {
        Arc::clone(&self.platform.authority)
    }

    pub fn adopt_inherited_writer() -> Result<ProcessDiscoveryInheritedWriter> {
        let value = inherited::writer_value()?;
        let writer = MacosProcessCapabilityChildWriter::from_transferred(inherited::descriptor()?)?;
        Ok(ProcessDiscoveryInheritedWriter { value, writer })
    }
}

fn lock_writer(writer: &Mutex<WriterState>) -> std::sync::MutexGuard<'_, WriterState> {
    writer
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/discovery/discovery_capability_macos_tests.rs"]
mod tests;
