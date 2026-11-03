use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use yolo_hub_jsonrpc_protocol::{Answer, Call, Id, decode};
use yolo_hub_jsonrpc_transport::encode;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn every_public_command_survives_real_transport_codec_with_the_same_delivery_identity() {
    for (command, reply) in samples() {
        let original = Call::from_request(request(command));
        let encoded = encode(&original).unwrap();
        let received: Call = decode(&encoded).unwrap();
        assert_eq!(received.id, original.id);
        assert_eq!(received.fields().command, original.fields().command);
        if matches!(reply, Reply::Attached { .. }) {
            // The terminal wire adapter sends this through metadata + bounded binary frames.
            assert!(Answer::new(&received, reply).is_err());
            continue;
        }
        let response = Answer::new(&received, reply.clone()).unwrap();
        let encoded = encode(&response).unwrap();
        let received: Answer = decode(&encoded).unwrap();
        assert_eq!(received.reply(&original), Ok(reply));
    }
}

#[test]
fn malformed_peer_correlation_cannot_become_a_valid_remote_rejection() {
    for (command, _) in samples() {
        let original = Call::from_request(request(command));
        let mut response = Answer::new(
            &original,
            Reply::Error {
                code: PublicError::Unauthorized,
            },
        )
        .unwrap();
        response.id = Some(Id::fresh());
        let encoded = encode(&response).unwrap();
        let decoded: Answer = decode(&encoded).unwrap();
        assert_eq!(decoded.reply(&original), Err(PublicError::InvalidRequest));
    }
}
