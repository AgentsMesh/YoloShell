use super::record::{INVALID, HookGenerationRecord};
use anyhow::{Context, Result, ensure};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use yolo_agent_contract::AgentKind;
use yolo_paths::identity::ObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationEvidence {
    pub kind: AgentKind,
    pub executable: String,
    pub provider_session_id: String,
    pub cwd: PathBuf,
    pub data_home: PathBuf,
    pub transcript: PathBuf,
    pub transcript_identity: Option<ObjectIdentity>,
    pub cwd_identity: ObjectIdentity,
    pub data_home_identity: ObjectIdentity,
}

impl GenerationEvidence {
    pub fn new(
        kind: AgentKind,
        executable: String,
        provider_session_id: String,
        cwd: PathBuf,
        data_home: PathBuf,
        transcript: PathBuf,
        transcript_identity: Option<ObjectIdentity>,
    ) -> Result<Self> {
        ensure!(
            !provider_session_id.is_empty()
                && !executable.is_empty()
                && cwd.is_absolute()
                && data_home.is_absolute()
                && transcript.is_absolute(),
            INVALID
        );
        let cwd_identity = directory_identity(&cwd)?;
        let data_home_identity = directory_identity(&data_home)?;
        let evidence = Self {
            kind,
            executable,
            provider_session_id,
            cwd,
            data_home,
            transcript,
            transcript_identity,
            cwd_identity,
            data_home_identity,
        };
        evidence.verify_own()?;
        Ok(evidence)
    }

    pub fn verify_own(&self) -> Result<bool> {
        if directory_identity(&self.cwd)? != self.cwd_identity
            || directory_identity(&self.data_home)? != self.data_home_identity
        {
            return Ok(false);
        }
        match self.transcript_identity {
            Some(identity) => Ok(file_identity(&self.transcript)? == Some(identity)),
            None => Ok(std::fs::symlink_metadata(&self.transcript)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)),
        }
    }
}

pub(super) fn verify(record: &HookGenerationRecord) -> Result<bool> {
    if record.revoked {
        return Ok(false);
    }
    if directory_identity(Path::new(&record.binding.cwd))? != record.cwd_identity
        || directory_identity(Path::new(&record.binding.data_home))? != record.data_home_identity
    {
        return Ok(false);
    }
    match record.transcript_identity {
        Some(identity) => Ok(file_identity(Path::new(&record.binding.evidence_path))?
            == Some(identity)),
        None => Ok(true),
    }
}

pub(crate) fn directory_identity(path: &Path) -> Result<ObjectIdentity> {
    identity(path, libc::O_DIRECTORY)
}

fn file_identity(path: &Path) -> Result<Option<ObjectIdentity>> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    Ok(Some(identity(path, libc::O_NOFOLLOW)?))
}

fn identity(path: &Path, required_flag: libc::c_int) -> Result<ObjectIdentity> {
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | required_flag);
    let file = options
        .open(path)
        .with_context(|| format!("open hook authority evidence {}", path.display()))?;
    let identity = ObjectIdentity::from_file(&file)?.ok_or_else(|| anyhow::anyhow!(INVALID))?;
    let named = options.open(path)?;
    ensure!(
        ObjectIdentity::from_file(&named)? == Some(identity),
        INVALID
    );
    Ok(identity)
}
