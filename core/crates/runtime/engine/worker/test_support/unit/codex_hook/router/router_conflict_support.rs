use super::process_support::{directory, executable};
use super::support::{Authority, Fixture};
use crate::agent::hooks::ProviderVersion;
use crate::codex_hooks::CodexAdmissionRecord;
use std::path::PathBuf;
use uuid::Uuid;

pub(super) const PROCESS_ID_DELTA: u32 = 1;
pub(super) const PROCESS_START_DELTA: u64 = 1;
pub(super) const PROVIDER_VERSION_DELTA: u16 = 1;
const ALTERNATE_CWD_DIRECTORY: &str = "other-project";
const ALTERNATE_DATA_HOME_DIRECTORY: &str = "other-provider-home";
const ALTERNATE_HOOK_FILE_NAME: &str = "other-codex-hook";
const ALTERNATE_REAL_FILE_NAME: &str = "other-codex-real";

#[derive(Clone, Copy)]
pub(super) enum ConflictField {
    Token,
    ProcessId,
    ProcessStart,
    Cwd,
    DataHome,
    Hook,
    Real,
}

impl Fixture {
    pub(super) fn changed_record(
        &self,
        authority: &Authority,
        field: ConflictField,
    ) -> CodexAdmissionRecord {
        let mut request = authority.request.clone();
        match field {
            ConflictField::Token => request.identity.hook_token = Uuid::new_v4(),
            ConflictField::ProcessId => request.identity.process.pid += PROCESS_ID_DELTA,
            ConflictField::ProcessStart => {
                request.identity.process.start_time += PROCESS_START_DELTA;
            }
            ConflictField::Cwd => request.cwd = self.alternate_directory(ALTERNATE_CWD_DIRECTORY),
            ConflictField::DataHome => {
                request.data_home = self.alternate_directory(ALTERNATE_DATA_HOME_DIRECTORY);
            }
            ConflictField::Hook => {
                request.hook_executable = self.alternate_executable(ALTERNATE_HOOK_FILE_NAME);
            }
            ConflictField::Real => {
                request.real_executable = self.alternate_executable(ALTERNATE_REAL_FILE_NAME);
            }
        }
        CodexAdmissionRecord::capture_with_route_revision(
            &request,
            authority.record.route_revision(),
        )
        .unwrap()
    }

    fn alternate_directory(&self, name: &str) -> PathBuf {
        directory(&self.root.path().canonicalize().unwrap().join(name))
    }

    fn alternate_executable(&self, name: &str) -> PathBuf {
        executable(&self.hook.parent().unwrap().join(name))
    }
}

impl Authority {
    pub(super) fn future_version_record(&self) -> anyhow::Result<CodexAdmissionRecord> {
        let mut request = self.request.clone();
        let version = request.provider_version;
        request.provider_version = ProviderVersion::new(
            version.major,
            version.minor + PROVIDER_VERSION_DELTA,
            version.patch,
        );
        CodexAdmissionRecord::capture_with_route_revision(&request, self.record.route_revision())
    }
}
