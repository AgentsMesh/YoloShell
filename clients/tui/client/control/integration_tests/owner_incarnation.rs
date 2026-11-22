use super::fixture::{Fixture, TEST_DEADLINE};
use std::{
    future::{Future, poll_fn},
    task::Poll,
};
use tokio::time::{Instant, timeout_at};
use yolo_tui_client_control::{ClientControl, inspect, status};

#[tokio::test]
async fn inspected_handle_cannot_detach_a_replacement_owner_at_the_same_state_path() {
    let fixture = Fixture::new(false);
    let deadline = Instant::now() + TEST_DEADLINE;
    let original_store = fixture.store();
    let mut original = ClientControl::bind(&original_store).await.unwrap();
    let original_descriptor = fixture.descriptor();
    let stale_handle = timeout_at(deadline, inspect(&fixture.path))
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    original.withdraw().unwrap();
    drop(original_store);
    timeout_at(deadline, original.finish())
        .await
        .unwrap()
        .unwrap();

    let replacement_store = fixture.store();
    let mut replacement = ClientControl::bind(&replacement_store).await.unwrap();
    let replacement_descriptor = fixture.descriptor();
    assert_ne!(
        replacement_descriptor.endpoint,
        original_descriptor.endpoint
    );
    assert_ne!(replacement_descriptor.token, original_descriptor.token);
    assert!(
        timeout_at(deadline, stale_handle.detach())
            .await
            .unwrap()
            .is_err()
    );
    assert!(
        timeout_at(deadline, status(&fixture.path))
            .await
            .unwrap()
            .unwrap()
    );
    {
        let mut notification = std::pin::pin!(replacement.detached());
        let observed = poll_fn(|context| Poll::Ready(notification.as_mut().poll(context))).await;
        assert!(observed.is_pending());
    }
    assert_eq!(fixture.descriptor(), replacement_descriptor);

    replacement.withdraw().unwrap();
    drop(replacement_store);
    timeout_at(deadline, replacement.finish())
        .await
        .unwrap()
        .unwrap();
}
