use crate::{
    harness::Harness, operations, peer::*, queries, query_constants::*, query_requests::*,
};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::{ControlError, RPC_TIMEOUT};
use yolo_hub_jsonrpc_transport::TransportError;
use yolo_hub_rpc_protocol::Command;

#[tokio::test]
async fn query_keeps_the_normal_rpc_deadline_without_poisoning_fast_control() {
    for handshake in [false, true] {
        let mut harness = Harness::start();
        let mut control = harness.accept().await;
        let generation = harness.ready().await;
        let command = query();
        assert_eq!(
            command.operation_timeout(std::time::Duration::ZERO),
            std::time::Duration::ZERO
        );
        assert_eq!(command.operation_timeout(RPC_TIMEOUT), RPC_TIMEOUT);
        harness.send(Some(generation), command.clone()).await;
        let mut query_peer = if handshake {
            operations::awaiting_handshake(&harness).await.0
        } else {
            let mut peer = harness.accept().await;
            assert_eq!(receive(&mut peer).await.fields().command, command);
            peer
        };
        tokio::time::pause();
        tokio::time::advance(RPC_TIMEOUT + QUERY_TIMEOUT_MARGIN).await;
        let (_, error) = queries::failure(&mut harness, generation, &command, true).await;
        tokio::time::resume();
        assert!(matches!(
            error,
            ControlError::Client(ClientError::Transport(TransportError::Deadline))
        ));
        eof(&mut query_peer).await;
        let hello = hello_reply(harness.endpoint.hub);
        operations::round_trip(
            &mut harness,
            &mut control,
            generation,
            Command::Hello {},
            hello,
        )
        .await;
        operations::cancel_without_replay(&mut harness).await;
        eof(&mut control).await;
    }
}
