use super::fixtures;
use crate::{lifecycle::selection_from_view, summary::summary};
use yolo_hub_identity::{HubId, RuntimeEpoch};
use yolo_hub_terminal_protocol::{PublicError, RuntimeStatus};
use yolo_runtime::model::TerminalRuntimeStatus;

#[test]
fn lifecycle_selection_keeps_digest_timestamp_status_and_runtime_fences() {
    let view = fixtures::view();
    let selected = selection_from_view(
        fixtures::target().address.hub_id,
        fixtures::command(),
        &view,
    )
    .unwrap();
    assert_eq!(selected.terminal_id, view.manifest.terminal_id);
    assert_eq!(
        selected.expected_updated_at_ms,
        Some(view.manifest.updated_at_ms)
    );
    assert_eq!(
        selected.expected_status,
        Some(TerminalRuntimeStatus::Running)
    );
    assert_eq!(
        selected.expected_runtime_epoch,
        Some(fixtures::target().runtime_epoch.as_uuid())
    );
    assert!(selected.expected_manifest_sha256.is_some());
    assert!(!selected.require_runtime_absent);
}

#[test]
fn changed_runtime_origin_and_no_runtime_expectations_fail_closed() {
    let view = fixtures::view();
    let mut command = fixtures::command();
    command.selection.expected_runtime = Some(RuntimeEpoch::new());
    assert_eq!(
        selection_from_view(fixtures::target().address.hub_id, command, &view),
        Err(PublicError::TargetMismatch)
    );
    command.selection.expected_runtime = None;
    assert_eq!(
        selection_from_view(fixtures::target().address.hub_id, command, &view),
        Err(PublicError::TargetMismatch)
    );
    assert_eq!(
        selection_from_view(HubId::new(), fixtures::command(), &view),
        Err(PublicError::TargetMismatch)
    );
    let mut absent = view;
    absent.runtime = None;
    let selected =
        selection_from_view(fixtures::target().address.hub_id, command, &absent).unwrap();
    assert!(selected.require_runtime_absent);
    command.request_id = uuid::Uuid::nil();
    assert_eq!(
        selection_from_view(fixtures::target().address.hub_id, command, &absent),
        Err(PublicError::InvalidRequest)
    );
}

#[test]
fn summary_status_mapping_never_turns_unreachable_into_exited() {
    for (internal, public) in [
        (TerminalRuntimeStatus::Starting, RuntimeStatus::Starting),
        (TerminalRuntimeStatus::Running, RuntimeStatus::Running),
        (TerminalRuntimeStatus::Finalizing, RuntimeStatus::Finalizing),
        (
            TerminalRuntimeStatus::OutputBlocked,
            RuntimeStatus::OutputBlocked,
        ),
        (TerminalRuntimeStatus::Exited, RuntimeStatus::Exited),
        (TerminalRuntimeStatus::Lost, RuntimeStatus::Lost),
        (
            TerminalRuntimeStatus::Unreachable,
            RuntimeStatus::Unreachable,
        ),
    ] {
        let mut view = fixtures::view();
        view.runtime.as_mut().unwrap().status = internal;
        let summary = summary(fixtures::target().address.hub_id, &view).unwrap();
        assert_eq!(summary.fields().runtime.unwrap().status, public);
    }
}

#[test]
fn summary_rejects_invalid_local_identity_and_nonunicode_directory() {
    let hub = fixtures::target().address.hub_id;
    let mut view = fixtures::view();
    view.manifest.terminal_id = uuid::Uuid::nil();
    assert_eq!(summary(hub, &view), Err(PublicError::Internal));
    let mut view = fixtures::view();
    view.runtime.as_mut().unwrap().runtime_epoch = uuid::Uuid::nil();
    assert_eq!(summary(hub, &view), Err(PublicError::Internal));
    let mut view = fixtures::view();
    view.manifest.name.clear();
    assert_eq!(summary(hub, &view), Err(PublicError::InvalidMetadata));
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        let mut view = fixtures::view();
        view.manifest.cwd = std::ffi::OsString::from_vec(vec![u8::MAX]).into();
        assert_eq!(summary(hub, &view), Err(PublicError::InvalidMetadata));
    }
}
