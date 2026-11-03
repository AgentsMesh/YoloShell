use super::*;
use std::sync::Arc;

#[tokio::test]
async fn unstarted_and_already_completed_projection_does_not_wait() {
    let completion = ProjectionCompletion::default();
    completion.wait_for(Duration::ZERO).await.unwrap();
    let ticket = completion.start().unwrap();
    assert!(completion.start().is_err());
    assert!(completion.wait_for(Duration::ZERO).await.is_err());
    ticket.finish(&Ok(()));
    completion.wait_for(Duration::ZERO).await.unwrap();
}

#[tokio::test]
async fn completion_waits_for_the_registered_projection() {
    let completion = Arc::new(ProjectionCompletion::default());
    let ticket = completion.start().unwrap();
    let waiting = Arc::clone(&completion);
    let task = tokio::spawn(async move { waiting.wait().await });
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    ticket.finish(&Ok(()));
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn cancelling_an_unpolled_projection_is_not_successful_completion() {
    let completion = ProjectionCompletion::default();
    let ticket = completion.start().unwrap();
    let task = tokio::spawn(async move {
        let _ticket = ticket;
        std::future::pending::<()>().await;
    });
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(
        completion
            .wait_for(Duration::ZERO)
            .await
            .unwrap_err()
            .to_string(),
        CODEX_PROJECTION_COMPLETION_CLOSED
    );
}

const PROJECTION_FAILURE: &str = "injected projection drain failure";

#[tokio::test]
async fn explicit_failure_is_shared_by_concurrent_and_later_waiters() {
    let completion = Arc::new(ProjectionCompletion::default());
    let ticket = completion.start().unwrap();
    let first = Arc::clone(&completion);
    let second = Arc::clone(&completion);
    let first = tokio::spawn(async move { first.wait().await });
    let second = tokio::spawn(async move { second.wait().await });
    ticket.finish(&Err(anyhow::anyhow!(PROJECTION_FAILURE)));
    for outcome in [
        first.await.unwrap(),
        second.await.unwrap(),
        completion.wait().await,
    ] {
        assert_eq!(outcome.unwrap_err().to_string(), PROJECTION_FAILURE);
    }
}

#[tokio::test]
async fn timed_out_waiter_does_not_consume_the_terminal_result() {
    let completion = ProjectionCompletion::default();
    let ticket = completion.start().unwrap();
    assert!(completion.wait_for(Duration::ZERO).await.is_err());
    ticket.finish(&Ok(()));
    completion.wait_for(Duration::ZERO).await.unwrap();
    completion.wait_for(Duration::ZERO).await.unwrap();
}

#[tokio::test]
async fn panicking_projection_reports_lost_completion_to_every_waiter() {
    let completion = ProjectionCompletion::default();
    let ticket = completion.start().unwrap();
    let task = tokio::spawn(async move {
        let _ticket = ticket;
        panic!("{PROJECTION_FAILURE}");
    });
    assert!(task.await.unwrap_err().is_panic());
    assert!(completion.wait().await.is_err());
    assert!(completion.wait().await.is_err());
}
