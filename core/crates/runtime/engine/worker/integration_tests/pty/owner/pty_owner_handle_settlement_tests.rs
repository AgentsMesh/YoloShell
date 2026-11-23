use super::*;
use std::sync::Arc;
use std::time::Duration;

const SETTLEMENT_FAILURE_DETAIL: &str = "injected settlement failure";
const SETTLEMENT_PROBE: Duration = Duration::from_millis(20);

#[tokio::test]
async fn settled_paths_report_the_authoritative_result() {
    let complete = settlement();
    complete.set(Ok(())).unwrap();
    let (sender, _receiver) = mpsc::channel();
    PtyOwnerHandle::for_test(sender, complete)
        .submit_termination(false)
        .unwrap()
        .wait()
        .await
        .unwrap();

    let failed = settlement();
    failed
        .set(Err(PtyOwnerCommandFailure::Owner(
            OWNER_FAILURE_DETAIL.to_owned(),
        )))
        .unwrap();
    let (sender, _receiver) = mpsc::channel();
    let error = PtyOwnerHandle::for_test(sender, failed)
        .submit_termination(false)
        .unwrap()
        .wait()
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), OWNER_FAILURE_DETAIL);
}

#[tokio::test]
async fn response_loss_waits_for_authoritative_settlement() {
    let settled = settlement();
    let (sender, receiver) = mpsc::channel();
    let submission = PtyOwnerHandle::for_test(sender, Arc::clone(&settled))
        .submit_termination(true)
        .unwrap();
    let PtyOwnerCommand::Terminate { response, .. } = receiver.recv().unwrap();
    drop(response);
    let mut wait = Box::pin(submission.wait());
    assert!(
        tokio::time::timeout(SETTLEMENT_PROBE, &mut wait)
            .await
            .is_err()
    );
    settled.set(Ok(())).unwrap();
    wait.await.unwrap();
}

#[tokio::test]
async fn ready_command_response_wins_over_conflicting_settlement() {
    for (response, published, expected_error) in [
        (
            Err(PtyOwnerCommandFailure::Owner(
                OWNER_FAILURE_DETAIL.to_owned(),
            )),
            Ok(()),
            Some(OWNER_FAILURE_DETAIL),
        ),
        (
            Ok(()),
            Err(PtyOwnerCommandFailure::Owner(
                SETTLEMENT_FAILURE_DETAIL.to_owned(),
            )),
            None,
        ),
    ] {
        let settled = settlement();
        let (sender, receiver) = mpsc::channel();
        let submission = PtyOwnerHandle::for_test(sender, Arc::clone(&settled))
            .submit_termination(true)
            .unwrap();
        let PtyOwnerCommand::Terminate {
            response: completion,
            ..
        } = receiver.recv().unwrap();
        completion.send(response).unwrap();
        settled.set(published).unwrap();

        let result = submission.wait().await;

        match expected_error {
            Some(detail) => assert_eq!(result.unwrap_err().to_string(), detail),
            None => result.unwrap(),
        }
    }
}

#[tokio::test]
async fn send_disconnect_waits_for_authoritative_deferred_settlement() {
    let settled = settlement();
    let publication = Arc::clone(&settled);
    let (sender, receiver) = mpsc::channel();
    let handle = PtyOwnerHandle::for_test(sender, settled);

    let submission = handle
        .submit_termination_after_for_test(false, move || {
            drop(receiver);
        })
        .unwrap();
    let mut wait = Box::pin(submission.wait());
    assert!(
        tokio::time::timeout(SETTLEMENT_PROBE, &mut wait)
            .await
            .is_err()
    );
    publication
        .set(Err(PtyOwnerCommandFailure::CleanupDeferred(
            crate::worker::termination_timing::CapabilitySettlementDeferred,
        )))
        .unwrap();
    let error = wait.await.unwrap_err();

    assert!(
        error
            .downcast_ref::<crate::worker::termination_timing::CapabilitySettlementDeferred>()
            .is_some()
    );
}
