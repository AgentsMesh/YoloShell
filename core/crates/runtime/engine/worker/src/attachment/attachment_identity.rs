pub(crate) const ATTACHMENT_SEQUENCE_INCREMENT: u64 = 1;
pub(crate) const ATTACHMENT_CURSOR_EXHAUSTED_CONTEXT: &str =
    "terminal attachment sequence cursor is exhausted";
pub const ATTACHMENT_CURSOR_AHEAD_CONTEXT: &str =
    "terminal attachment cursor is ahead of durable output";

#[derive(Debug)]
pub(crate) enum AttachmentResultError {
    CursorAhead,
    CursorExhausted,
    Internal(anyhow::Error),
}

impl AttachmentResultError {
    pub(crate) fn into_error_body(self) -> ErrorBody {
        match self {
            Self::CursorAhead => ErrorBody::invalid(ATTACHMENT_CURSOR_AHEAD_CONTEXT),
            Self::CursorExhausted => ErrorBody::invalid(ATTACHMENT_CURSOR_EXHAUSTED_CONTEXT),
            Self::Internal(error) => internal_error(error),
        }
    }
}

impl From<anyhow::Error> for AttachmentResultError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl std::fmt::Display for AttachmentResultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CursorAhead => formatter.write_str(ATTACHMENT_CURSOR_AHEAD_CONTEXT),
            Self::CursorExhausted => formatter.write_str(ATTACHMENT_CURSOR_EXHAUSTED_CONTEXT),
            Self::Internal(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

#[doc(hidden)]
pub use yolo_runtime_protocol::attachment_next_sequence;

pub(crate) const fn attachment_observed_sequence(next_sequence: u64) -> Option<u64> {
    next_sequence.checked_sub(ATTACHMENT_SEQUENCE_INCREMENT)
}

pub(crate) fn resynchronized_attachment(
    snapshot: TerminalSnapshot,
) -> std::result::Result<AttachResult, AttachmentResultError> {
    let next_sequence = attachment_next_sequence(snapshot.barrier_sequence)
        .ok_or(AttachmentResultError::CursorExhausted)?;
    Ok(AttachResult {
        runtime_epoch: snapshot.runtime_epoch,
        next_sequence,
        snapshot: Some(snapshot),
        chunks: Vec::new(),
        resynchronized: true,
    })
}


#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-attachment"))]
#[path = "../../unit_tests/attachment/attachment_identity_tests.rs"]
mod attachment_identity_tests;
