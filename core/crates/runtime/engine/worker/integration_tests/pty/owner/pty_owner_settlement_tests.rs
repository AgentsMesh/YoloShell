use super::*;
use std::future::Future;
use std::sync::mpsc;
use std::task::Poll;
use std::time::Duration;

const FIRST_FAILURE_DETAIL: &str = "first owner settlement";
const SECOND_FAILURE_DETAIL: &str = "second owner settlement";
const RACING_FAILURE_DETAIL: &str = "settlement published before command send";
const RACING_SETTLEMENT_TIMEOUT_DETAIL: &str =
    "termination submission ignored published settlement";
const LATER_WAITER_TIMEOUT_DETAIL: &str = "later settlement waiter was not notified";
const WAITER_PENDING_TIMEOUT_DETAIL: &str = "settlement waiter did not register before publication";
const CONCURRENT_WAITER_COUNT: usize = 4;
const PENDING_WAIT_PROBE: Duration = Duration::from_millis(20);
const SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn publisher_drop_publishes_an_owner_failure() {
    let (settlement, publisher) = new_owner_settlement();
    drop(publisher);

    let completion = settlement.wait().await;
    let failure = completion.unwrap_err();
    assert!(failure.contains(OWNER_SETTLEMENT_ABANDONED_DETAIL));
}

#[tokio::test]
async fn handle_created_after_publisher_drop_observes_abandonment() {
    let (settlement, publisher) = new_owner_settlement();
    drop(publisher);
    let (commands, _receiver) = mpsc::channel();
    let handle = super::super::PtyOwnerHandle::for_test(commands, settlement);

    let error = handle.submit_termination(false).unwrap().wait().await;

    assert!(
        error
            .unwrap_err()
            .to_string()
            .contains(OWNER_SETTLEMENT_ABANDONED_DETAIL)
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn publisher_drop_wakes_every_concurrent_waiter() {
    let (settlement, publisher) = new_owner_settlement();
    let mut waiters = Vec::with_capacity(CONCURRENT_WAITER_COUNT);
    let mut pending = Vec::with_capacity(CONCURRENT_WAITER_COUNT);
    for _ in 0..CONCURRENT_WAITER_COUNT {
        let observer = Arc::clone(&settlement);
        let (registered, registration) = tokio::sync::oneshot::channel();
        pending.push(registration);
        waiters.push(tokio::spawn(async move {
            let mut wait = Box::pin(observer.wait());
            let mut registered = Some(registered);
            std::future::poll_fn(|context| match wait.as_mut().poll(context) {
                Poll::Pending => {
                    if let Some(registered) = registered.take() {
                        let _ = registered.send(());
                    }
                    Poll::Pending
                }
                Poll::Ready(completion) => Poll::Ready(completion),
            })
            .await
        }));
    }
    for registration in pending {
        tokio::time::timeout(SETTLEMENT_TIMEOUT, registration)
            .await
            .expect(WAITER_PENDING_TIMEOUT_DETAIL)
            .unwrap();
    }

    drop(publisher);

    for waiter in waiters {
        let completion = tokio::time::timeout(SETTLEMENT_TIMEOUT, waiter)
            .await
            .expect(LATER_WAITER_TIMEOUT_DETAIL)
            .unwrap();
        assert!(matches!(completion, Err(PtyOwnerCommandFailure::Owner(_))));
    }
}

#[test]
fn first_publication_remains_authoritative_after_publisher_drop() {
    let (settlement, publisher) = new_owner_settlement();
    settlement
        .set(Err(PtyOwnerCommandFailure::Owner(
            FIRST_FAILURE_DETAIL.to_owned(),
        )))
        .unwrap();
    let second = settlement.set(Err(PtyOwnerCommandFailure::Owner(
        SECOND_FAILURE_DETAIL.to_owned(),
    )));
    drop(publisher);

    assert!(second.is_err());
    let failure = settlement.get().unwrap().as_ref().unwrap_err();
    assert!(failure.contains(FIRST_FAILURE_DETAIL));
    assert!(!failure.contains(SECOND_FAILURE_DETAIL));
}

#[tokio::test]
async fn publication_between_precheck_and_send_wins_for_both_send_outcomes() {
    for send_succeeds in [true, false] {
        let (settlement, publisher) = new_owner_settlement();
        let publication = Arc::clone(&settlement);
        let (sender, receiver) = mpsc::channel();
        let receiver = send_succeeds.then_some(receiver);
        let handle = super::super::PtyOwnerHandle::for_test(sender, Arc::clone(&settlement));

        let submission = handle
            .submit_termination_after_for_test(false, move || {
                assert!(publication.get().is_none());
                publication
                    .set(Err(PtyOwnerCommandFailure::Owner(
                        RACING_FAILURE_DETAIL.to_owned(),
                    )))
                    .unwrap();
            })
            .unwrap();
        let pending = receiver.as_ref().map(|receiver| receiver.recv().unwrap());
        assert_eq!(pending.is_some(), send_succeeds);

        let error = tokio::time::timeout(SETTLEMENT_TIMEOUT, submission.wait())
            .await
            .expect(RACING_SETTLEMENT_TIMEOUT_DETAIL)
            .unwrap_err();
        assert_eq!(error.to_string(), RACING_FAILURE_DETAIL);
        drop(pending);
        drop(publisher);
    }
}

#[tokio::test]
async fn cancelling_one_waiter_does_not_consume_later_publication() {
    let settlement = new_owner_settlement_for_test();
    let mut cancelled = Box::pin(settlement.wait());
    assert!(
        tokio::time::timeout(PENDING_WAIT_PROBE, &mut cancelled)
            .await
            .is_err()
    );
    drop(cancelled);

    settlement.set(Ok(())).unwrap();

    let completion = tokio::time::timeout(SETTLEMENT_TIMEOUT, settlement.wait())
        .await
        .expect(LATER_WAITER_TIMEOUT_DETAIL);
    assert_eq!(completion, Ok(()));
}
