use super::*;
#[path = "../test_support/query_constants.rs"]
mod query_constants;
#[path = "../test_support/query_requests.rs"]
mod query_requests;
use query_constants::*;
use tokio::{io::AsyncReadExt, net::UnixStream, sync::oneshot};

fn control_request() -> ControlRequest {
    ControlRequest {
        validity: None,
        generation: Some(Uuid::new_v4()),
        destination: yolo_hub_identity::HubId::new(),
        command: query_requests::query(),
    }
}

#[tokio::test]
async fn panic_retains_the_exact_request_and_closes_only_its_query_socket() {
    let mut queries = Queries::default();
    let request = control_request();
    let other = control_request();
    let expected = query_requests::ready(&other.command, other.destination);
    let (owned, mut peer) = UnixStream::pair().unwrap();
    let (trigger, triggered) = oneshot::channel::<()>();
    queries.spawn(&request, async move {
        let _owned = owned;
        triggered.await.unwrap();
        panic!("{QUERY_TASK_PANIC}")
    });
    let (complete, completion) = oneshot::channel();
    queries.spawn(&other, async move { completion.await.unwrap() });
    trigger.send(()).unwrap();
    let failed = tokio::time::timeout(QUERY_TEST_TIMEOUT, queries.next())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(failed.result, Err(ControlError::QueryTaskFailed)));
    assert_eq!(failed.request.command, request.command);
    assert_eq!(failed.request.generation, request.generation);
    assert_eq!(failed.request.destination, request.destination);
    assert_eq!(
        tokio::time::timeout(QUERY_TEST_TIMEOUT, peer.read_u8())
            .await
            .unwrap()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::UnexpectedEof
    );
    assert!(!queries.is_empty());
    complete.send(Ok(expected.clone())).unwrap();
    let survived = queries.next().await.unwrap();
    assert_eq!(survived.result.unwrap(), expected);
    assert!(queries.is_empty() && queries.requests.is_empty());
}

#[tokio::test]
async fn an_aborted_query_task_is_reported_once_with_its_retained_scope() {
    let mut queries = Queries::default();
    let scope = crate::RequestScope::default();
    let mut request = control_request();
    let validity = scope.validity_until(std::time::Instant::now() + QUERY_TEST_TIMEOUT);
    request.validity = Some(validity.clone());
    let (owned, mut peer) = UnixStream::pair().unwrap();
    queries.spawn(&request, async move {
        let _owned = owned;
        std::future::pending().await
    });
    queries.tasks.abort_all();
    let failed = queries.next().await.unwrap();
    assert!(matches!(failed.result, Err(ControlError::QueryTaskFailed)));
    assert!(failed.request.validity.unwrap().same_scope(&validity));
    assert_eq!(failed.request.command, request.command);
    assert!(queries.next().await.is_none());
    assert_eq!(
        tokio::time::timeout(QUERY_TEST_TIMEOUT, peer.read_u8())
            .await
            .unwrap()
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}
