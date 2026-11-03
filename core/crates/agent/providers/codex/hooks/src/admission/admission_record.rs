#[path = "admission_executable.rs"]
mod executable;

use super::CodexAdmissionContext;
use crate::agent::hooks::{EventKind, EventSequence, HookEnvelope, HookIdentity, HookProviderKind};
use anyhow::{Result, ensure};
pub use executable::{CodexExecutableAttestation, CodexExecutableIdentity, CodexExecutableKind};
use serde::{Deserialize, Serialize};

const CODEX_ADMISSION_SCHEMA_VERSION: u16 = 4;
const ADMISSION_INVALID: &str = "Codex invocation admission record is invalid";

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_executable_bounds_tests.rs"]
mod bounds_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_digest_tests.rs"]
mod digest_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/admission/admission_record_tests.rs"]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexAdmissionRecord {
    schema_version: u16,
    route_revision: u64,
    request: CodexAdmissionContext,
    first_event: HookEnvelope,
    hook_executable: CodexExecutableAttestation,
    real_executable: CodexExecutableAttestation,
}

impl CodexAdmissionRecord {
    pub fn capture_native_with_route_revision(
        request: CodexAdmissionContext,
        route_revision: u64,
        first_event: HookEnvelope,
    ) -> Result<Self> {
        request.validate()?;
        let record = Self {
            schema_version: CODEX_ADMISSION_SCHEMA_VERSION,
            route_revision,
            hook_executable: CodexExecutableAttestation::capture(
                &request.hook_executable,
                CodexExecutableKind::Runtime,
            )?,
            real_executable: CodexExecutableAttestation::capture(
                &request.real_executable,
                CodexExecutableKind::Provider,
            )?,
            request,
            first_event,
        };
        record.validate_for(&record.request.identity)?;
        Ok(record)
    }

    pub(crate) fn validate_for(&self, expected: &HookIdentity) -> Result<()> {
        ensure!(
            self.schema_version == CODEX_ADMISSION_SCHEMA_VERSION
                && self.route_revision != u64::default()
                && &self.request.identity == expected,
            ADMISSION_INVALID
        );
        self.request.validate()?;
        let first = &self.first_event;
        ensure!(
            first.identity == self.request.identity
                && first.provider == HookProviderKind::Codex
                && first.provider_version == self.request.provider_version
                && first.metadata.cwd.as_ref() == Some(&self.request.cwd)
                && !first.event_id.is_nil()
                && first.sequence == EventSequence::ZERO
                && first.agent_id.is_none()
                && matches!(first.event, EventKind::SessionStart { .. }),
            ADMISSION_INVALID
        );
        super::outbox_envelope_validation::validate(first)?;
        let mut initial = first.clone();
        initial.sequence = EventSequence::FIRST;
        let mut reducer = crate::agent::hooks::Reducer::new(
            expected.clone(),
            HookProviderKind::Codex,
            self.request.provider_version,
        )
        .map_err(anyhow::Error::new)?;
        ensure!(
            reducer.apply(initial).map_err(anyhow::Error::new)?
                == crate::agent::hooks::ApplyDisposition::Applied,
            ADMISSION_INVALID
        );
        self.hook_executable
            .validate_for(&self.request.hook_executable, CodexExecutableKind::Runtime)?;
        self.real_executable
            .validate_for(&self.request.real_executable, CodexExecutableKind::Provider)
    }

    pub(crate) fn verify_executables(&self) -> Result<()> {
        self.hook_executable
            .verify_current(CodexExecutableKind::Runtime)?;
        self.real_executable
            .verify_current(CodexExecutableKind::Provider)
    }

    pub fn request(&self) -> &CodexAdmissionContext {
        &self.request
    }
    pub fn route_revision(&self) -> u64 {
        self.route_revision
    }
    pub fn first_event(&self) -> &HookEnvelope {
        &self.first_event
    }
    pub fn hook_executable(&self) -> &CodexExecutableAttestation {
        &self.hook_executable
    }
    pub(crate) fn real_executable(&self) -> &CodexExecutableAttestation {
        &self.real_executable
    }

    pub fn verify_native_processes(&self, agent_pid: u32, helper_pid: u32) -> Result<()> {
        self.verify_native_agent()?;
        ensure!(
            agent_pid == self.request.identity.process.pid,
            ADMISSION_INVALID
        );
        self.hook_executable
            .verify_mapped_process(helper_pid, CodexExecutableKind::Runtime)?;
        self.verify_executables()
    }

    pub fn verify_native_agent(&self) -> Result<()> {
        let process = self.request.identity.process;
        ensure!(
            crate::process::ProcessIdentity {
                pid: process.pid,
                start_time: process.start_time
            }
            .matches(),
            ADMISSION_INVALID
        );
        self.verify_executables()?;
        self.real_executable
            .verify_mapped_process(process.pid, CodexExecutableKind::Provider)?;
        Ok(())
    }
}
