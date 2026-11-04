use super::*;
use tokio::io::{AsyncWriteExt, duplex};
use yolo_hub_enrollment_wire::{
    JOIN_SCHEMA_VERSION, JoinErrorCode, JoinOutcome, JoinReply, WireError, read_outcome,
    read_request, write_outcome, write_request,
};

#[tokio::test]
async fn join_request_and_outcome_round_trip() {
    let (mut left, mut right) = duplex(4096);
    let request = request();
    write_request(&mut left, &request).await.unwrap();
    assert_eq!(read_request(&mut right).await.unwrap(), request);

    let reply = JoinReply {
        schema_version: JOIN_SCHEMA_VERSION,
        hub: peer(),
        revision: 3,
        applied: true,
    };
    write_outcome(&mut left, &JoinOutcome::Accepted { reply })
        .await
        .unwrap();
    assert_eq!(
        read_outcome(&mut right).await.unwrap(),
        JoinOutcome::Accepted { reply }
    );
}

#[tokio::test]
async fn rejection_codes_round_trip() {
    let (mut left, mut right) = duplex(4096);
    let outcome = JoinOutcome::Rejected {
        code: JoinErrorCode::Unauthorized,
    };
    write_outcome(&mut left, &outcome).await.unwrap();
    assert_eq!(read_outcome(&mut right).await.unwrap(), outcome);
}

#[tokio::test]
async fn malformed_and_oversized_frames_are_rejected() {
    let (mut left, mut right) = duplex(64);
    left.write_u32(u32::MAX).await.unwrap();
    assert!(matches!(
        read_request(&mut right).await,
        Err(WireError::Size)
    ));

    let (mut left, mut right) = duplex(64);
    left.write_u32(2).await.unwrap();
    left.write_all(b"{}").await.unwrap();
    assert!(matches!(
        read_request(&mut right).await,
        Err(WireError::Codec)
    ));
}

#[tokio::test]
async fn invalid_requests_are_rejected_before_framing() {
    let mut invalid = request();
    invalid.request_id = Uuid::nil();
    let (mut left, _right) = duplex(64);
    assert!(matches!(
        write_request(&mut left, &invalid).await,
        Err(WireError::Rejected(JoinErrorCode::InvalidRequest))
    ));
}
