use crate::fixtures::{self, constants::*, roundtrip};
use yolo_hub_identity::{HubId, TerminalId};
use yolo_hub_terminal_protocol::*;

#[test]
fn summary_has_only_portable_resource_metadata() {
    let summary = TerminalSummary::new(fixtures::summary_fields()).unwrap();
    assert_eq!(summary.fields().address, fixtures::address());
    roundtrip(&summary);
    let mut fields: TerminalSummaryFields = summary.into();
    fields.runtime = None;
    fields.agent = None;
    fields.archived = true;
    roundtrip(&TerminalSummary::new(fields).unwrap());
}

#[test]
fn unreachable_observation_preserves_confirmed_exit_and_never_creates_a_runtime() {
    for status in [
        RuntimeStatus::Running,
        RuntimeStatus::Exited,
        RuntimeStatus::Lost,
    ] {
        let mut fields = fixtures::summary_fields();
        fields.runtime.as_mut().unwrap().status = status;
        let mut summary = TerminalSummary::new(fields).unwrap();
        summary.mark_unreachable();
        let expected = if status == RuntimeStatus::Running {
            RuntimeStatus::Unreachable
        } else {
            status
        };
        assert_eq!(summary.fields().runtime.unwrap().status, expected);
        assert_eq!(summary.fields().runtime.unwrap().target, fixtures::target());
    }
    let mut fields = fixtures::summary_fields();
    fields.runtime = None;
    let mut summary = TerminalSummary::new(fields).unwrap();
    summary.mark_unreachable();
    assert!(summary.fields().runtime.is_none());
}

#[test]
fn summary_rejects_cross_origin_and_cross_session_children() {
    let mut fields = fixtures::summary_fields();
    fields.runtime.as_mut().unwrap().target.address.terminal_id = TerminalId::new();
    assert_eq!(
        TerminalSummary::new(fields),
        Err(PublicError::TargetMismatch)
    );
    let mut fields = fixtures::summary_fields();
    fields.agent.as_mut().unwrap().address.hub_id = HubId::new();
    assert_eq!(
        TerminalSummary::new(fields),
        Err(PublicError::OriginMismatch)
    );
}

#[test]
fn summary_checks_each_metadata_boundary() {
    for text in INVALID_TEXT {
        let mut fields = fixtures::summary_fields();
        fields.name = text.into();
        assert_eq!(
            TerminalSummary::new(fields),
            Err(PublicError::InvalidMetadata)
        );
        let mut fields = fixtures::summary_fields();
        fields.cwd = text.into();
        assert_eq!(
            TerminalSummary::new(fields),
            Err(PublicError::InvalidMetadata)
        );
    }
    let mut fields = fixtures::summary_fields();
    fields.name = NAME.repeat(MAX_TERMINAL_NAME_BYTES);
    assert!(TerminalSummary::new(fields).is_err());
    let mut fields = fixtures::summary_fields();
    fields.cwd = NAME.repeat(MAX_DIRECTORY_BYTES);
    assert!(TerminalSummary::new(fields).is_err());
}

#[test]
fn every_runtime_status_and_public_error_roundtrips() {
    for status in [
        RuntimeStatus::Starting,
        RuntimeStatus::Running,
        RuntimeStatus::Finalizing,
        RuntimeStatus::OutputBlocked,
        RuntimeStatus::Exited,
        RuntimeStatus::Lost,
        RuntimeStatus::Unreachable,
    ] {
        roundtrip(&status);
    }
    for error in [
        PublicError::UnsupportedSchema,
        PublicError::InvalidRequest,
        PublicError::InvalidDirectory,
        PublicError::RecreateRequired,
        PublicError::InvalidSize,
        PublicError::InvalidMetadata,
        PublicError::InvalidCapability,
        PublicError::InvalidLease,
        PublicError::InvalidPayload,
        PublicError::InvalidCursor,
        PublicError::OriginMismatch,
        PublicError::TargetMismatch,
        PublicError::OutputGap,
        PublicError::NotFound,
        PublicError::Unauthorized,
        PublicError::Unreachable,
        PublicError::Conflict,
        PublicError::LimitExceeded,
        PublicError::Unsupported,
        PublicError::OutcomeUnknown,
        PublicError::Internal,
    ] {
        roundtrip(&error);
        assert!(!error.to_string().is_empty());
    }
}
