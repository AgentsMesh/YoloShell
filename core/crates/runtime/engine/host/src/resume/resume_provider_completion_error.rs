use super::*;

pub(crate) const RESUME_PROVIDER_COMPLETION_PENDING: &str =
    "provider resume completion evidence is not yet authoritative";

#[derive(Debug)]
pub(crate) enum ProviderCompletionError {
    Pending(anyhow::Error),
    Contradiction(anyhow::Error),
}

impl ProviderCompletionError {
    pub(crate) fn pending(message: impl std::fmt::Display) -> Self {
        Self::Pending(anyhow::anyhow!(message.to_string()))
    }

    pub(crate) fn contradiction(error: anyhow::Error) -> Self {
        Self::Contradiction(error)
    }

    #[cfg(test)]
    pub(crate) fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    pub(crate) fn into_resume_failure(
        self,
        host: &Host,
        manifest: &mut TerminalManifest,
    ) -> SessionResumeFailure {
        match self {
            Self::Pending(error) => ErrorBody::new(
                crate::protocol::error_code::CONFLICT,
                format!("{RESUME_PROVIDER_COMPLETION_PENDING}: {error}"),
                true,
            )
            .into(),
            Self::Contradiction(error) => host.resume_transaction_failure(manifest, error),
        }
    }
}

impl std::fmt::Display for ProviderCompletionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending(error) | Self::Contradiction(error) => error.fmt(formatter),
        }
    }
}

pub(crate) type ProviderCompletionResult<T> = std::result::Result<T, ProviderCompletionError>;
