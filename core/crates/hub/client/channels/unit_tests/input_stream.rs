use super::{
    commands::{grant, input},
    constants::TIMEOUT,
};
use crate::{
    ControlError, ControlRequest, InputStream,
    input_completion::{Dispatch, settle},
};
use std::time::Instant;
use uuid::Uuid;
use yolo_hub_client::ClientError;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

fn request(scope: &crate::RequestScope) -> ControlRequest {
    let hub = HubId::new();
    ControlRequest {
        generation: Some(Uuid::new_v4()),
        destination: hub,
        command: input(grant(hub)),
        validity: Some(scope.validity_until(Instant::now() + TIMEOUT)),
    }
}

#[test]
fn view_retirement_and_local_rejection_do_not_poison_other_views() {
    for error in [
        ControlError::Invalidated,
        ControlError::NotConnected,
        ControlError::StaleGeneration,
    ] {
        let stream = InputStream::new(Instant::now() + TIMEOUT);
        let previous = stream.view();
        let pending = request(&previous);
        let current = stream.view();
        let suffix = request(&current);
        drop(previous);
        settle(&pending, &Err(error), Dispatch::NotAttempted);
        assert!(!pending.validity.as_ref().unwrap().is_current());
        assert!(suffix.validity.as_ref().unwrap().is_current());
        assert!(stream.is_current());
    }
}

#[test]
fn attempted_failure_fences_existing_and_future_views_without_waiting_for_ui_receipts() {
    for result in [
        Err(ControlError::Client(ClientError::Remote(
            PublicError::OutcomeUnknown,
        ))),
        Err(ControlError::Client(ClientError::OutcomeUnknownResponse(
            PublicError::InvalidPayload,
        ))),
        Ok(Reply::Completed {}),
    ] {
        let stream = InputStream::new(Instant::now() + TIMEOUT);
        let previous = stream.view();
        let pending = request(&previous);
        drop(previous);
        let current = stream.view();
        let suffix = request(&current);
        settle(&pending, &result, Dispatch::Attempted);
        assert!(!suffix.validity.as_ref().unwrap().is_current());
        assert!(
            !stream
                .view()
                .validity_until(Instant::now() + TIMEOUT)
                .is_current()
        );
        assert!(!stream.renew_authority(Instant::now() + TIMEOUT));
        assert!(!current.renew_authority(Instant::now() + TIMEOUT));
    }
}

#[test]
fn stream_replacement_expiry_and_owner_drop_never_retarget_old_failure() {
    let original = InputStream::new(Instant::now() + TIMEOUT);
    let old_view = original.view();
    let old = request(&old_view);
    let replacement = InputStream::new(Instant::now() + TIMEOUT);
    let view = replacement.view();
    let current = request(&view);
    drop(original);
    assert!(!old.validity.as_ref().unwrap().is_current());
    old.validity.as_ref().unwrap().reject_dispatched_input();
    assert!(current.validity.as_ref().unwrap().is_current());
    assert!(replacement.renew_authority(Instant::now() + TIMEOUT));
    let expired = InputStream::new(Instant::now());
    assert!(!expired.renew_authority(Instant::now() + TIMEOUT));
    assert!(!expired.is_current());
}
