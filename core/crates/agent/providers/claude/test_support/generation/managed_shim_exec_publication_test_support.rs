use super::super::{ClaudeGenerationAuthority, HookPublicationAdmission, HookPublicationFence};
pub(super) use super::managed_shim_exec_publication_fixture::{Case, path_text};
use crate::agent::{AgentBinding, AgentBindingStore};
use anyhow::Result;
use std::path::PathBuf;

const SHIM_DIRECTORY: &str = "bin";
const FOREIGN_SESSION_DIRECTORY: &str = "foreign-session";
const DEADLINE_DECREMENT_NS: u64 = 1;

impl Case {
    pub fn accepts_at(
        &self,
        authority: &ClaudeGenerationAuthority,
        successor: &crate::agent::ObservedProcess,
        predecessor: &AgentBinding,
        boot_id: &str,
        now: u64,
    ) -> bool {
        super::super::managed_shim_exec_publication::accepts_for_test(
            authority,
            successor,
            predecessor,
            &self.shim,
            boot_id,
            now,
        )
    }

    pub fn accepts(&self) -> bool {
        self.accepts_at(
            &self.authority,
            &self.successor,
            &self.predecessor,
            &self.authority.handshake_boot_id,
            self.authority.handshake_deadline_ns - DEADLINE_DECREMENT_NS,
        )
    }

    pub fn foreign_shim(&self) -> PathBuf {
        self.shim
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(FOREIGN_SESSION_DIRECTORY)
            .join(SHIM_DIRECTORY)
            .join(crate::agent::claude::constants::EXECUTABLE)
    }

    pub fn admission(
        &self,
        authority: &ClaudeGenerationAuthority,
        allow_special: bool,
    ) -> Result<HookPublicationAdmission> {
        let binding = self.binding_at();
        if allow_special {
            HookPublicationAdmission::for_predecessor(
                authority,
                self.successor.clone(),
                Some(&binding),
                &self.shim,
                AcceptingFence,
            )
        } else {
            HookPublicationAdmission::new(authority, self.successor.clone(), AcceptingFence)
        }
    }

    pub fn publish(
        &self,
        authority: &ClaudeGenerationAuthority,
    ) -> Result<(ClaudeGenerationAuthority, AgentBinding)> {
        self.publish_with_force(authority, false)
    }

    pub fn publish_with_force(
        &self,
        authority: &ClaudeGenerationAuthority,
        force: bool,
    ) -> Result<(ClaudeGenerationAuthority, AgentBinding)> {
        let admission = self.admission(authority, true)?;
        self.fixture.store.publish_hook_binding_state_in_namespace(
            &self.fixture.lease,
            &self.fixture.binding_path(),
            self.fixture.authentication(),
            authority,
            force,
            &admission,
            |_| Ok(()),
        )
    }

    pub fn binding_at(&self) -> AgentBinding {
        AgentBindingStore::new(self.fixture.binding_path())
            .read_raw_in_namespace(&self.fixture.lease)
            .unwrap()
            .unwrap()
    }

    pub fn write_binding(&self, binding: &AgentBinding) {
        self.fixture
            .lease
            .write_json(&self.fixture.binding_path(), binding)
            .unwrap();
    }
}

pub(super) struct AcceptingFence;

impl HookPublicationFence for AcceptingFence {
    fn verify(&self, _: &ClaudeGenerationAuthority) -> Result<()> {
        Ok(())
    }
}
