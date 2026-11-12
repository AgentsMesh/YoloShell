use super::*;

pub(crate) const EXPLICIT_AUTOMATIC_BLOCK_DETAIL: &str =
    "an automatic-only agent recovery block reached explicit resume";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AutomaticResumeBlock {
    RecoveryEvidenceChanged,
    UserTermination,
}

#[derive(Debug)]
pub(crate) enum SessionResumeFailure {
    TerminalRecreate(ErrorBody),
    Protocol(ErrorBody),
    AutomaticBlocked(AutomaticResumeBlock),
}

impl SessionResumeFailure {
    pub(crate) fn into_explicit_error(self) -> ErrorBody {
        match self {
            Self::TerminalRecreate(error) | Self::Protocol(error) => error,
            Self::AutomaticBlocked(_) => ErrorBody::internal(EXPLICIT_AUTOMATIC_BLOCK_DETAIL),
        }
    }

    /// True when automatic reconciliation must not retry this failure.
    pub(crate) fn is_terminal(&self) -> bool {
        match self {
            Self::TerminalRecreate(error) | Self::Protocol(error) => !error.retryable,
            Self::AutomaticBlocked(_) => false,
        }
    }
}

impl From<ErrorBody> for SessionResumeFailure {
    fn from(error: ErrorBody) -> Self {
        Self::Protocol(error)
    }
}

impl Host {
    pub(crate) async fn persist_classified_resume_failure(
        &self,
        manifest: &mut TerminalManifest,
        state: RecoveryState,
        failure: SessionResumeFailure,
    ) -> SessionResumeFailure {
        match self.persist_recovery_state_required(manifest, state).await {
            Ok(()) => failure,
            Err(error) => {
                if let Some(committed) =
                    error.downcast_ref::<recovery_manifest_publication::RecoveryCatalogFailure>()
                {
                    let error = match committed.downgrade(self, manifest.terminal_id) {
                        Ok(current) => {
                            *manifest = current;
                            error
                        }
                        Err(downgrade) => error.context(downgrade),
                    };
                    return self.resume_transaction_failure_body(manifest.terminal_id, error);
                }
                self.resume_transaction_failure(manifest, error)
            }
        }
    }
}
