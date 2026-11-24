use super::{ClaudeNativeGeneration, INVALID};
use crate::paths::identity::ObjectIdentity;
use anyhow::{Result, ensure};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub(super) fn directory_identity(path: &Path) -> Result<ObjectIdentity> {
    crate::paths::validation::validate_absolute(path)?;
    ensure!(
        path.canonicalize()? == crate::paths::platform::platform_path(path),
        INVALID
    );
    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY);
    let file = options.open(path)?;
    let identity = ObjectIdentity::from_file(&file)?.ok_or_else(|| anyhow::anyhow!(INVALID))?;
    let named = options.open(path)?;
    ensure!(
        ObjectIdentity::from_file(&named)? == Some(identity),
        INVALID
    );
    Ok(identity)
}

pub(super) fn verify(record: &ClaudeNativeGeneration) -> Result<bool> {
    if record.revoked {
        return Ok(false);
    }
    let binding = &record.binding;
    let cwd = Path::new(&binding.cwd);
    let data_home = Path::new(&binding.data_home);
    let transcript = Path::new(&binding.evidence_path);
    if directory_identity(cwd)? != record.cwd_identity
        || directory_identity(data_home)? != record.data_home_identity
    {
        return Ok(false);
    }
    let Some(evidence) = crate::claude::ClaudeAdapter.inspect_authoritative_evidence_with_hook(
        transcript,
        cwd,
        data_home,
        || {},
    )?
    else {
        return Ok(false);
    };
    Ok(evidence.identity() == record.transcript_identity
        && evidence.evidence().provider_session_id == binding.provider_session_id
        && evidence.evidence().cwd == cwd
        && evidence.evidence().data_home == data_home
        && evidence.verify()?
        && directory_identity(cwd)? == record.cwd_identity
        && directory_identity(data_home)? == record.data_home_identity)
}
