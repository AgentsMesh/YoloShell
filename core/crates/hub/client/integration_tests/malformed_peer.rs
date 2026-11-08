use crate::{
    ClientError,
    fixtures::{constants::*, *},
    server::*,
};
use tokio::io::AsyncWriteExt;
use yolo_hub_jsonrpc_protocol::Answer as Response;
use yolo_hub_jsonrpc_transport::{FRAME_DELIMITER, MAX_FRAME_BYTES, encode};
use yolo_hub_rpc_protocol::Reply;

#[tokio::test]
async fn every_truncated_response_prefix_poisoned_after_mutation_never_becomes_a_rejection() {
    let expected = yolo_hub_jsonrpc_protocol::Call::new(hub(), input()).unwrap();
    let encoded =
        encode(&Response::new(&expected, Reply::Input { duplicate: false }).unwrap()).unwrap();
    for prefix in ONE_BYTE..encoded.len() {
        let (mut client, mut server) = connected().await;
        let peer = async {
            let received = receive(&mut server).await;
            assert_eq!(received.fields().command, expected.fields().command);
            let encoded =
                encode(&Response::new(&received, Reply::Input { duplicate: false }).unwrap())
                    .unwrap();
            let mut stream = server.into_stream().unwrap();
            stream.write_all(&encoded[..prefix]).await.unwrap();
        };
        let (result, ()) = tokio::join!(client.call(hub(), input(), deadline()), peer);
        let error = result.err().unwrap();
        assert!(error.outcome_is_unknown());
        assert!(!client.is_usable());
        assert!(matches!(
            client.call(hub(), input(), deadline()).await,
            Err(ClientError::Poisoned)
        ));
    }
}

#[tokio::test]
async fn empty_oversized_and_non_response_frames_are_not_valid_operation_results() {
    let invalid_frames = [
        vec![FRAME_DELIMITER],
        vec![b' '; MAX_FRAME_BYTES + ONE_BYTE],
        encode(&SAMPLE_PAYLOAD).unwrap(),
    ];
    for bytes in invalid_frames {
        for mutation in [false, true] {
            let (mut client, mut server) = connected().await;
            let peer = async {
                let _request = receive(&mut server).await;
                let mut stream = server.into_stream().unwrap();
                stream.write_all(&bytes).await.unwrap();
            };
            let command = if mutation { input() } else { list() };
            let (result, ()) = tokio::join!(client.call(hub(), command, deadline()), peer);
            assert_eq!(result.err().unwrap().outcome_is_unknown(), mutation);
            assert!(!client.is_usable());
        }
    }
}
