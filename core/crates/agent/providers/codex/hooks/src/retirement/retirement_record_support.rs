use super::*;

pub(super) fn process_identity(identity: &HookIdentity) -> crate::process::ProcessIdentity {
    crate::process::ProcessIdentity {
        pid: identity.process.pid,
        start_time: identity.process.start_time,
    }
}

pub(super) fn admission_digest(admission: &CodexAdmissionAuthority) -> Result<Sha256Digest> {
    Sha256Digest::capture(&(ADMISSION_DIGEST_DOMAIN, admission.record()))
}

pub(super) fn seal_digest(seal: &CodexRetirementSeal) -> Result<Sha256Digest> {
    Sha256Digest::capture(&(SEAL_DIGEST_DOMAIN, seal))
}

pub(super) fn snapshot_digest(snapshot: &ReducerSnapshot) -> Result<Sha256Digest> {
    Sha256Digest::capture(&(SNAPSHOT_DIGEST_DOMAIN, snapshot))
}

pub(super) fn disposition(reducer: &Reducer) -> Result<CodexRetirementDisposition> {
    let state = reducer.state();
    if state.process == ProcessState::Exited {
        return Ok(if state.final_hook_seen {
            CodexRetirementDisposition::CleanFinal
        } else {
            CodexRetirementDisposition::Abandoned
        });
    }
    if state.revoked {
        return Ok(CodexRetirementDisposition::Revoked);
    }
    if reducer.authority_exhausted() {
        return Ok(CodexRetirementDisposition::AuthorityExhausted);
    }
    Err(corrupt())
}

pub(super) fn corrupt() -> anyhow::Error {
    anyhow::anyhow!(CORRUPT)
}
