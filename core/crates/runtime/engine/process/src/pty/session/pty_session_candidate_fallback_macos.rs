use super::super::macos::EnvironmentCapability;
use super::super::macos_contract::{CapabilityProofError, CapabilityProofResult};

pub(super) const CAPABILITY_QUERY_FAILURE_CONTEXT: &str =
    "candidate with the captured PTY device had an unreadable capability";
pub(super) const DESCRIPTOR_QUERY_FAILURE_CONTEXT: &str =
    "fallback PTY descriptor query failed after capability query failure";

pub(in crate::process::pty_session_proof) fn match_tty_if_capability<T>(
    capability: EnvironmentCapability,
    tty: T,
) -> CapabilityProofResult<bool>
where
    T: FnOnce() -> CapabilityProofResult<bool>,
{
    capability
        .matches
        .then(tty)
        .transpose()
        .map(Option::unwrap_or_default)
}

pub(in crate::process::pty_session_proof) fn match_tty_after_environment<T>(
    environment: CapabilityProofResult<EnvironmentCapability>,
    tty: T,
) -> CapabilityProofResult<bool>
where
    T: FnOnce() -> CapabilityProofResult<bool>,
{
    match environment {
        Ok(capability) => match_tty_if_capability(capability, tty),
        Err(CapabilityProofError::CandidateLocalQuery(environment_error)) => match tty() {
            Ok(false) => Ok(false),
            Ok(true) => Err(CapabilityProofError::candidate(
                environment_error.context(CAPABILITY_QUERY_FAILURE_CONTEXT),
            )),
            Err(CapabilityProofError::CandidateLocalQuery(descriptor_error)) => {
                let context = format!("{DESCRIPTOR_QUERY_FAILURE_CONTEXT}: {environment_error:#}");
                Err(CapabilityProofError::candidate(
                    descriptor_error.context(context),
                ))
            }
            Err(error) => Err(error),
        },
        Err(error) => Err(error),
    }
}

#[cfg(test)]
pub(in crate::process::pty_session_proof) fn match_environment_if_tty<E>(
    has_tty: bool,
    environment: E,
) -> CapabilityProofResult<bool>
where
    E: FnOnce() -> CapabilityProofResult<EnvironmentCapability>,
{
    if !has_tty {
        return Ok(false);
    }
    environment().map(|capability| capability.matches)
}
