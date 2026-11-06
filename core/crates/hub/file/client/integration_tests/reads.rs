use crate::{constants::*, fixture::*};
use yolo_hub_file_client::{FileClientError, read};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn bytes_are_read_through_the_shared_client_using_the_exact_origin_target() {
    let (_, _, roots) = roots(1);
    let request = read_request(&roots[0]);
    let (mut client, server) = connected(vec![Reply::File {
        file: file(&roots[0]),
    }])
    .await;
    assert_eq!(
        read(&mut client, request.clone(), deadline())
            .await
            .unwrap()
            .fields()
            .data,
        PAYLOAD
    );
    assert_eq!(server.await.unwrap(), vec![Command::ReadFile { request }]);
}

#[tokio::test]
async fn a_rejected_read_is_not_replayed_or_converted_into_empty_success() {
    let (_, _, roots) = roots(1);
    let (mut client, server) = connected(vec![Reply::Error {
        code: PublicError::Unauthorized,
    }])
    .await;
    assert!(matches!(
        read(&mut client, read_request(&roots[0]), deadline()).await,
        Err(FileClientError::Client(
            yolo_hub_client::ClientError::Remote(PublicError::Unauthorized)
        ))
    ));
    assert_eq!(server.await.unwrap().len(), 1);
}
