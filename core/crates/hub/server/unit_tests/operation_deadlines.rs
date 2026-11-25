#[path = "../test_support/operation_deadlines.rs"]
mod support;
use crate::{constants::OPERATION_TIMEOUT, operation};
use support::*;
use tokio::time::{Instant, timeout};
use yolo_hub_dispatch::Operation;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::PublicError;
use yolo_session_timing::SESSION_STARTUP_CLIENT_TIMEOUT;

#[tokio::test(start_paused = true)]
async fn creation_dispatch_can_finish_at_the_host_startup_boundary_without_replay() {
    let dispatcher = DelayedDispatcher::new(SESSION_STARTUP_CLIENT_TIMEOUT);
    let request = dispatcher.creation();
    let started = Instant::now();
    let Operation::Reply(reply) = operation::dispatch(&dispatcher, &request).await.unwrap() else {
        panic!("creation returned a terminal attachment")
    };
    assert_eq!(Ok(reply), dispatcher.result);
    assert!(started.elapsed() > OPERATION_TIMEOUT);
    assert_eq!(started.elapsed(), SESSION_STARTUP_CLIENT_TIMEOUT);
    dispatcher.assert_counts(SINGLE_CALL);
}

#[tokio::test(start_paused = true)]
async fn expired_creation_cancels_dispatch_and_preserves_uncertain_mutation_outcome() {
    let mut dispatcher = DelayedDispatcher::new(SESSION_STARTUP_CLIENT_TIMEOUT);
    let request = dispatcher.creation();
    let budget = request
        .fields()
        .command
        .operation_timeout(OPERATION_TIMEOUT);
    dispatcher.delay = budget + OPERATION_TIMEOUT;
    let started = Instant::now();
    assert!(matches!(
        operation::dispatch(&dispatcher, &request).await,
        Err(PublicError::OutcomeUnknown)
    ));
    assert_eq!(started.elapsed(), budget);
    dispatcher.assert_counts(NO_COMPLETIONS);
}

#[tokio::test(start_paused = true)]
async fn read_only_build_keeps_its_short_deadline_and_unreachable_outcome() {
    let dispatcher = DelayedDispatcher::new(SESSION_STARTUP_CLIENT_TIMEOUT);
    let request = Request::new(dispatcher.hub, Command::GetBuild {}).unwrap();
    let started = Instant::now();
    assert!(matches!(
        operation::dispatch(&dispatcher, &request).await,
        Err(PublicError::Unreachable)
    ));
    assert_eq!(started.elapsed(), OPERATION_TIMEOUT);
    dispatcher.assert_counts(NO_COMPLETIONS);
}

#[tokio::test(start_paused = true)]
async fn caller_cancellation_drops_long_dispatch_without_waiting_for_its_extended_budget() {
    let dispatcher = DelayedDispatcher::new(SESSION_STARTUP_CLIENT_TIMEOUT);
    let request = dispatcher.creation();
    assert!(
        timeout(
            OPERATION_TIMEOUT,
            operation::dispatch(&dispatcher, &request)
        )
        .await
        .is_err()
    );
    dispatcher.assert_counts(NO_COMPLETIONS);
}

#[tokio::test(start_paused = true)]
async fn completed_dispatch_preserves_the_origins_explicit_error() {
    let mut dispatcher = DelayedDispatcher::new(OPERATION_TIMEOUT);
    dispatcher.result = Err(PublicError::Unauthorized);
    assert!(matches!(
        operation::dispatch(&dispatcher, &dispatcher.creation()).await,
        Err(PublicError::Unauthorized)
    ));
    dispatcher.assert_counts(SINGLE_CALL);
}
