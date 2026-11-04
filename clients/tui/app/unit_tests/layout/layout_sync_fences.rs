use super::layout_support::*;
use crate::layout_sync::{complete, pump, refresh};
use uuid::Uuid;
use yolo_hub_rpc_protocol::Reply;
use yolo_presentation_layout::Axis;

#[test]
fn stale_or_forged_operation_cannot_settle_current_request() {
    let (mut state, jobs, mut incoming) = fixture();
    refresh(&mut state);
    let operation = request(&mut state, &jobs, &mut incoming);
    let mut forged = operation.clone();
    forged.token = Uuid::new_v4();
    let changed = remote(&state);
    response(&mut state, forged, changed);
    assert!(state.shared_layout.as_ref().unwrap().pending.is_some());
    let mut forged = operation.clone();
    forged.command = yolo_hub_rpc_protocol::Command::ListLayouts {};
    complete(&mut state, forged, Ok(Reply::Completed {}));
    assert!(state.shared_layout.as_ref().unwrap().pending.is_some());
    let original = state.shared_layout.as_ref().unwrap().document.clone();
    response(&mut state, operation, original);
    assert!(state.shared_layout.as_ref().unwrap().pending.is_none());
}

#[test]
fn wrong_identity_revision_or_reply_keeps_local_document_and_requests_resync() {
    for case in 0..4 {
        let (mut state, jobs, mut incoming) = fixture();
        let original = state.shared_layout.as_ref().unwrap().document.clone();
        refresh(&mut state);
        let operation = request(&mut state, &jobs, &mut incoming);
        let mut corrupt = original.clone();
        match case {
            0 => corrupt.address.layout_id = Uuid::new_v4(),
            1 => corrupt.revision = 0,
            2 => corrupt.tabs[0].title = "unversioned modification".into(),
            _ => corrupt.last_request_id = Uuid::nil(),
        }
        response(&mut state, operation, corrupt);
        let sync = state.shared_layout.as_ref().unwrap();
        assert_eq!(sync.document, original);
        assert!(sync.read);
    }
    for reply in [Reply::Completed {}, Reply::Layout { document: None }] {
        let (mut state, jobs, mut incoming) = fixture();
        refresh(&mut state);
        let operation = request(&mut state, &jobs, &mut incoming);
        complete(&mut state, operation, Ok(reply));
        assert!(state.shared_layout.as_ref().unwrap().read);
    }
}

#[test]
fn write_acknowledgement_must_match_exact_submitted_tabs_and_request_id() {
    let (mut state, jobs, mut incoming) = fixture();
    state.tab_layout_mut().split(Axis::Vertical, None).unwrap();
    let operation = request(&mut state, &jobs, &mut incoming);
    let mut fake = applied(&state, &operation);
    fake.last_request_id = Uuid::new_v4();
    response(&mut state, operation, fake);
    assert!(state.shared_layout.as_ref().unwrap().uncertain.is_some());
}

#[test]
fn unavailable_or_full_lane_preserves_unsent_edits() {
    let (mut state, jobs, mut incoming) = fixture();
    state.tab_layout_mut().split(Axis::Vertical, None).unwrap();
    assert!(
        jobs.try_send(crate::network::Job::Refresh {
            request_id: Uuid::new_v4()
        })
        .is_ok()
    );
    pump(&mut state, &jobs);
    assert!(state.shared_layout.as_ref().unwrap().pending.is_none());
    incoming.try_recv().unwrap();
    let operation = request(&mut state, &jobs, &mut incoming);
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
    let document = applied(&state, &operation);
    state.shared_layout = None;
    response(&mut state, operation, document);
    refresh(&mut state);
    pump(&mut state, &jobs);
    assert!(incoming.try_recv().is_err());
}
