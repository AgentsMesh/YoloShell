use super::{CodexHookRejection, VerifiedLiveOccurrence};

pub(super) fn verify_occurrence(
    occurrence: Option<&VerifiedLiveOccurrence>,
) -> Result<(), CodexHookRejection> {
    occurrence
        .map(VerifiedLiveOccurrence::verify)
        .transpose()
        .map(drop)
        .map_err(durable_rejection)
}

pub(super) fn process_rejection(error: anyhow::Error) -> CodexHookRejection {
    CodexHookRejection::Process(error)
}

pub(super) fn durable_rejection(error: anyhow::Error) -> CodexHookRejection {
    CodexHookRejection::Storage(error)
}

pub(super) fn storage_rejection(
    error: super::super::super::router::CodexRouterError,
) -> CodexHookRejection {
    CodexHookRejection::Storage(anyhow::Error::new(error))
}
