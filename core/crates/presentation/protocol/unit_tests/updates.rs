use super::{fixture::*, *};
use uuid::Uuid;

#[test]
fn create_then_cas_closes_views_without_changing_terminal_identity() {
    let mut request = update();
    let first = request.apply(None).unwrap();
    let terminal = first.tabs[0].root.panes()[0].terminal;
    request.expected_revision = first.revision;
    request.request_id = Uuid::new_v4();
    request.tabs[0].title.push_str(" review");
    let next = request.apply(Some(&first)).unwrap();
    assert_eq!(next.tabs[0].root.panes()[0].terminal, terminal);
    assert_eq!(next.last_request_id, request.request_id);
    assert_eq!(
        request.apply(Some(&next)),
        Err(ProtocolError::RevisionConflict)
    );
    request.expected_revision = next.revision;
    request.tabs.clear();
    assert!(request.apply(Some(&next)).unwrap().tabs.is_empty());
}

#[test]
fn mismatch_missing_update_and_exhaustion_are_rejected_without_a_result() {
    let mut request = update();
    let first = request.apply(None).unwrap();
    assert_eq!(
        request.apply(Some(&first)),
        Err(ProtocolError::RevisionConflict)
    );
    request.expected_revision = first.revision;
    assert_eq!(request.apply(None), Err(ProtocolError::RevisionConflict));
    let mut foreign = first.clone();
    foreign.address.layout_id = Uuid::new_v4();
    assert_eq!(
        request.apply(Some(&foreign)),
        Err(ProtocolError::AddressMismatch)
    );
    foreign.revision = 0;
    assert_eq!(
        request.apply(Some(&foreign)),
        Err(ProtocolError::InvalidRevision)
    );
    request.expected_revision = u64::MAX;
    assert_eq!(request.validate(), Err(ProtocolError::InvalidRevision));
}

#[test]
fn every_protocol_error_is_a_useful_public_error() {
    for error in [
        ProtocolError::InvalidId,
        ProtocolError::InvalidTitle,
        ProtocolError::InvalidTree,
        ProtocolError::Limit,
        ProtocolError::DuplicateId,
        ProtocolError::InvalidRevision,
        ProtocolError::RevisionConflict,
        ProtocolError::AddressMismatch,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }
}
