use super::constants::*;
use super::{CodexExecutableAttestation, CodexHookInput, CodexNativeRequest};
use crate::codex_hooks::CodexExecutableKind;
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const ENVIRONMENT_INVALID: &str = "Codex native hook environment is invalid";

pub(super) struct NativeEnvironment {
    pub(super) socket: PathBuf,
    pub(super) executable: CodexExecutableAttestation,
    pub(super) terminal_id: Uuid,
    pub(super) runtime_epoch: Uuid,
    pub(super) data_home_candidates: Vec<PathBuf>,
}

impl NativeEnvironment {
    pub(super) fn capture(paths: &crate::paths::Paths) -> Result<Self> {
        Self::from_values(paths, &std::env::current_exe()?, |name| {
            std::env::var_os(name)
        })
    }

    pub(super) fn from_values(
        paths: &crate::paths::Paths,
        current_executable: &Path,
        get: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Self> {
        let home = required_path(get(NATIVE_HOOK_HOME_ENV))?;
        ensure!(home == paths.home(), ENVIRONMENT_INVALID);
        let executable = required_path(get(NATIVE_HOOK_EXECUTABLE_ENV))?;
        ensure!(
            current_executable.canonicalize()? == executable,
            ENVIRONMENT_INVALID
        );
        let executable =
            CodexExecutableAttestation::capture(&executable, CodexExecutableKind::Runtime)?;
        let data_home_candidates = data_homes(&get)?;
        Ok(Self {
            socket: required_path(get(SOCKET_ENV))?,
            executable,
            terminal_id: required_uuid(get(TERMINAL_ENV))?,
            runtime_epoch: required_uuid(get(RUNTIME_ENV))?,
            data_home_candidates,
        })
    }

    pub(super) fn request(&self, input: CodexHookInput) -> Result<CodexNativeRequest> {
        self.executable
            .verify_current(CodexExecutableKind::Runtime)?;
        let request = CodexNativeRequest {
            schema_version: NATIVE_SCHEMA_VERSION,
            terminal_id: self.terminal_id,
            runtime_epoch: self.runtime_epoch,
            event_id: Uuid::new_v4(),
            hook_executable: self.executable.path().to_path_buf(),
            data_home_candidates: self.data_home_candidates.clone(),
            input,
        };
        request.validate()?;
        Ok(request)
    }
}

fn data_homes(get: &impl Fn(&str) -> Option<OsString>) -> Result<Vec<PathBuf>> {
    let mut candidates = Vec::new();
    if let Some(home) = get(NATIVE_DATA_HOME_ENV).filter(|value| !value.is_empty()) {
        let home = PathBuf::from(home);
        crate::paths::validation::validate_absolute(&home)?;
        candidates.push(home);
    }
    if let Some(home) = get(NATIVE_USER_HOME_ENV) {
        let home = PathBuf::from(home).join(NATIVE_DATA_DIRECTORY_NAME);
        crate::paths::validation::validate_absolute(&home)?;
        if !candidates.contains(&home) {
            candidates.push(home);
        }
    }
    ensure!(!candidates.is_empty(), ENVIRONMENT_INVALID);
    Ok(candidates)
}

fn required_path(value: Option<OsString>) -> Result<PathBuf> {
    let path = PathBuf::from(value.context(ENVIRONMENT_INVALID)?);
    crate::paths::validation::validate_absolute(&path)?;
    Ok(path)
}

fn required_uuid(value: Option<OsString>) -> Result<Uuid> {
    let value = value.context(ENVIRONMENT_INVALID)?;
    let value = value.to_str().context(ENVIRONMENT_INVALID)?;
    let id = Uuid::parse_str(value).context(ENVIRONMENT_INVALID)?;
    ensure!(!id.is_nil() && id.to_string() == value, ENVIRONMENT_INVALID);
    Ok(id)
}
