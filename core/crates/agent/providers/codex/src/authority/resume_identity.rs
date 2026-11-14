use crate::agent::{AgentBinding, AgentKind};
use crate::paths::identity::ObjectIdentity;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

const INVALID: &str = "Codex resume identity no longer matches verified provider evidence";
const SINGLE_LINK: u64 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CodexResumeIdentity {
    pub(super) binding: AgentBinding,
    pub(super) boot_id: String,
    transcript: ObjectIdentity,
    directory: ObjectIdentity,
    data_home: ObjectIdentity,
}

impl CodexResumeIdentity {
    pub(super) fn capture(binding: &AgentBinding, boot_id: &str) -> Result<Self> {
        let value = Self {
            binding: binding.clone(),
            boot_id: boot_id.to_owned(),
            transcript: object(Path::new(&binding.evidence_path), false, true)?,
            // The working directory is shared by the user and the project; it
            // may legitimately be owned by another account (for example /tmp
            // or a root-owned checkout). The transcript and provider home
            // remain required to be user-owned.
            directory: object(Path::new(&binding.cwd), true, false)?,
            data_home: object(Path::new(&binding.data_home), true, true)?,
        };
        value.validate_for(binding)?;
        Ok(value)
    }

    pub(super) fn validate_for(&self, binding: &AgentBinding) -> Result<()> {
        ensure!(
            !self.boot_id.is_empty()
                && self.binding.kind == AgentKind::Codex
                && self.binding.schema_version == binding.schema_version
                && self.binding.terminal_id == binding.terminal_id
                && self.binding.provider_session_id == binding.provider_session_id
                && self.binding.executable == binding.executable
                && self.binding.cwd == binding.cwd
                && self.binding.data_home == binding.data_home
                && self.binding.evidence_path == binding.evidence_path
                && self.binding.detected_at_ms == binding.detected_at_ms
                && self.binding.updated_at_ms <= binding.updated_at_ms,
            INVALID
        );
        Ok(())
    }

    pub(super) fn verify(&self) -> Result<()> {
        ensure!(
            object(Path::new(&self.binding.evidence_path), false, true)? == self.transcript
                && object(Path::new(&self.binding.cwd), true, false)? == self.directory
                && object(Path::new(&self.binding.data_home), true, true)? == self.data_home,
            INVALID
        );
        let evidence = crate::codex::CodexAdapter
            .inspect_evidence_with_hook(
                Path::new(&self.binding.evidence_path),
                Path::new(&self.binding.cwd),
                Path::new(&self.binding.data_home),
                || {},
            )?
            .ok_or_else(|| anyhow::anyhow!(INVALID))?;
        ensure!(
            evidence.provider_session_id == self.binding.provider_session_id
                && evidence.path == Path::new(&self.binding.evidence_path)
                && evidence.data_home == Path::new(&self.binding.data_home),
            INVALID
        );
        Ok(())
    }

    pub(super) fn same_source(&self, other: &Self) -> bool {
        self.boot_id == other.boot_id
            && self.transcript == other.transcript
            && self.directory == other.directory
            && self.data_home == other.data_home
    }

    pub(super) fn allows_successor(&self, other: &Self) -> bool {
        self.boot_id == other.boot_id
            && self.binding.evidence_path == other.binding.evidence_path
            && self.binding.data_home == other.binding.data_home
            && self.data_home == other.data_home
            && self.transcript == other.transcript
            && (self.binding.cwd != other.binding.cwd || self.directory == other.directory)
    }
}

fn object(path: &Path, directory: bool, require_owner: bool) -> Result<ObjectIdentity> {
    crate::paths::validation::validate_absolute(path)?;
    ensure!(
        path.canonicalize()? == crate::paths::platform::platform_path(path),
        INVALID
    );
    let flags = libc::O_NOFOLLOW | libc::O_CLOEXEC | if directory { libc::O_DIRECTORY } else { 0 };
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        owner_allowed(metadata.uid(), unsafe { libc::geteuid() }, require_owner)
            && if directory {
                metadata.is_dir()
            } else {
                metadata.is_file() && metadata.nlink() == SINGLE_LINK
            },
        INVALID
    );
    let identity = ObjectIdentity::from_file(&file)?.ok_or_else(|| anyhow::anyhow!(INVALID))?;
    let named = OpenOptions::new()
        .read(true)
        .custom_flags(flags)
        .open(path)?;
    ensure!(
        ObjectIdentity::from_file(&named)? == Some(identity),
        INVALID
    );
    Ok(identity)
}

pub(super) fn owner_allowed(actual: u32, current: u32, require_owner: bool) -> bool {
    !require_owner || actual == current
}
