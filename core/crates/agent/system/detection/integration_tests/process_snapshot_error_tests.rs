use super::test_support::{envelope, frame, observed_wire, root};
use super::*;
use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::PathBuf;

const TEST_FAILURE_FILL: &str = "x";
const TEST_TRAILING_BYTE: u8 = 0xa5;
const TEST_TRUNCATED_PAYLOAD_LENGTH: u32 = 2;
const TEST_TRUNCATED_PAYLOAD_BYTE: u8 = 0x80;
const TEST_MALFORMED_PAYLOAD_LENGTH: u32 = 1;
const TEST_MALFORMED_PAYLOAD_BYTE: u8 = 0xc1;
const TEST_ENVIRONMENT_KEY_PREFIX: &str = "YOLO_TEST_KEY_";
const TEST_ABSOLUTE_PATH: &str = "/tmp/evidence";
const MASSIVE_EMPTY_ARGV_ITEMS: usize = 1_000_000;

#[tokio::test]
async fn snapshot_rejects_wrong_schema_or_root_identity() {
    let mut invalid = envelope(ProcessSnapshotWireOutcome::RootChanged);
    invalid.schema_version = PROCESS_SNAPSHOT_SCHEMA_VERSION + 1;
    assert_invalid_envelope(&invalid).await;

    invalid.schema_version = PROCESS_SNAPSHOT_SCHEMA_VERSION;
    invalid.root_pid += 1;
    assert_invalid_envelope(&invalid).await;

    invalid.root_pid = root().pid;
    invalid.root_start_time += 1;
    assert_invalid_envelope(&invalid).await;
}

#[tokio::test]
async fn snapshot_rejects_an_out_of_range_matching_root_identity() {
    let mut invalid_root = root();
    invalid_root.pid = u32::MAX;
    let mut invalid = envelope(ProcessSnapshotWireOutcome::RootChanged);
    invalid.root_pid = invalid_root.pid;
    let mut input = Cursor::new(crate::protocol::encode(&invalid).unwrap());

    assert!(
        read_process_snapshot(&mut input, invalid_root, Vec::new())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn snapshot_rejects_invalid_scan_timestamps() {
    let mut invalid = envelope(ProcessSnapshotWireOutcome::RootChanged);
    invalid.started_at_ms = i64::default();
    assert_invalid_envelope(&invalid).await;

    invalid.started_at_ms = invalid.completed_at_ms + 1;
    assert_invalid_envelope(&invalid).await;
}

#[tokio::test]
async fn snapshot_rejects_invalid_failure_detail_and_trailing_data() {
    for detail in [
        String::new(),
        TEST_FAILURE_FILL.repeat(MAX_PROCESS_SNAPSHOT_ERROR_BYTES + 1),
    ] {
        let mut input = Cursor::new(frame(ProcessSnapshotWireOutcome::Failed { detail }));
        assert!(read_process_snapshot(&mut input, root(), Vec::new()).await.is_err());
    }
    let mut bytes = frame(ProcessSnapshotWireOutcome::RootChanged);
    bytes.push(TEST_TRAILING_BYTE);
    assert!(
        read_process_snapshot(&mut Cursor::new(bytes), root(), Vec::new())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn snapshot_rejects_truncated_malformed_and_oversized_frames() {
    let mut truncated = TEST_TRUNCATED_PAYLOAD_LENGTH.to_be_bytes().to_vec();
    truncated.push(TEST_TRUNCATED_PAYLOAD_BYTE);
    let mut malformed = TEST_MALFORMED_PAYLOAD_LENGTH.to_be_bytes().to_vec();
    malformed.push(TEST_MALFORMED_PAYLOAD_BYTE);
    let oversized = ((super::limits::MAX_PROCESS_SNAPSHOT_FRAME_BYTES + 1) as u32)
        .to_be_bytes()
        .to_vec();
    for bytes in [truncated, malformed, oversized] {
        assert!(
            read_process_snapshot(&mut Cursor::new(bytes), root(), Vec::new())
                .await
                .is_err()
        );
    }
}

#[test]
fn snapshot_rejects_a_million_empty_arguments_before_frame_publication() {
    let mut process = observed_wire();
    process.argv = vec![String::new(); MASSIVE_EMPTY_ARGV_ITEMS];
    let invalid = envelope(ProcessSnapshotWireOutcome::Stable {
        processes: vec![process],
    });
    // Cap'n Proto accounts for each list element even when its text is empty.
    // This payload exceeds the wire budget before it can become a valid frame;
    // the smaller per-collection decode limits are exercised separately below.
    assert!(matches!(
        crate::protocol::encode(&invalid),
        Err(crate::protocol::ProtocolError::FrameTooLarge { .. })
    ));
}

#[tokio::test]
async fn snapshot_rejects_trailing_data_inside_the_declared_payload() {
    let mut bytes = frame(ProcessSnapshotWireOutcome::RootChanged);
    bytes.push(TEST_TRAILING_BYTE);
    let payload_bytes = bytes.len() - crate::protocol::FRAME_LENGTH_BYTES;
    let payload_bytes = u32::try_from(payload_bytes).unwrap().to_be_bytes();
    bytes[..crate::protocol::FRAME_LENGTH_BYTES].copy_from_slice(&payload_bytes);

    assert!(
        read_process_snapshot(&mut Cursor::new(bytes), root(), Vec::new())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn snapshot_rejects_each_oversized_collection_during_decode() {
    let mut argv = observed_wire();
    argv.argv = (0..=super::limits::MAX_PROCESS_SNAPSHOT_ARGV_ITEMS)
        .map(|_| String::new())
        .collect();
    assert_invalid_outcome(ProcessSnapshotWireOutcome::Stable {
        processes: vec![argv],
    })
    .await;

    let mut evidence = observed_wire();
    evidence.evidence_paths = Some(
        (0..=super::limits::MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS)
            .map(|_| PathBuf::from(TEST_ABSOLUTE_PATH).into())
            .collect(),
    );
    assert_invalid_outcome(ProcessSnapshotWireOutcome::Stable {
        processes: vec![evidence],
    })
    .await;

    let mut environment = observed_wire();
    environment.environment = Some(
        (0..=super::limits::MAX_PROCESS_SNAPSHOT_ENVIRONMENT_ITEMS)
            .map(|index| {
                (
                    format!("{TEST_ENVIRONMENT_KEY_PREFIX}{index}"),
                    String::new(),
                )
            })
            .collect::<BTreeMap<_, _>>(),
    );
    assert_invalid_outcome(ProcessSnapshotWireOutcome::Stable {
        processes: vec![environment],
    })
    .await;

    let processes = (0..=super::limits::MAX_PROCESS_SNAPSHOT_PROCESSES)
        .map(|index| {
            let mut process = observed_wire();
            process.pid += u32::try_from(index).unwrap();
            process
        })
        .collect();
    assert_invalid_outcome(ProcessSnapshotWireOutcome::Stable { processes }).await;
}

async fn assert_invalid_outcome(outcome: ProcessSnapshotWireOutcome) {
    assert!(
        read_process_snapshot(&mut Cursor::new(frame(outcome)), root(), Vec::new())
            .await
            .is_err()
    );
}

async fn assert_invalid_envelope(envelope: &ProcessSnapshotEnvelope) {
    let mut input = Cursor::new(crate::protocol::encode(envelope).unwrap());
    assert!(read_process_snapshot(&mut input, root(), Vec::new()).await.is_err());
}
