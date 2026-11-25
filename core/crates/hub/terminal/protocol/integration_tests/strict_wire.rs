use crate::fixtures::{self, constants::*, reject_field};
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;
use yolo_hub_terminal_protocol::*;

fn rejects_unknown<T: Serialize + DeserializeOwned>(value: T) {
    reject_field(&value, UNKNOWN_FIELD, UNKNOWN_VALUE.into());
}

#[test]
fn every_public_envelope_rejects_unknown_fields() {
    rejects_unknown(TerminalSize::default());
    rejects_unknown(CreateShellRequest::new(fixtures::create_fields()).unwrap());
    rejects_unknown(CreateShellResult {
        address: fixtures::address(),
        creation_capability: SecretCapability::generate(),
    });
    rejects_unknown(TerminalSummary::new(fixtures::summary_fields()).unwrap());
    rejects_unknown(fixtures::summary_fields().runtime.unwrap());
    rejects_unknown(TerminalSelection {
        address: fixtures::address(),
        expected_runtime: None,
    });
    rejects_unknown(TerminalCommand {
        request_id: Uuid::from_u128(REQUEST_ID),
        selection: TerminalSelection {
            address: fixtures::address(),
            expected_runtime: None,
        },
        action: TerminalAction::Archive {},
    });
    rejects_unknown(fixtures::principal());
    rejects_unknown(LeaseGrant::new(fixtures::grant_fields()).unwrap());
    rejects_unknown(LeaseRequest {
        request_id: Uuid::from_u128(REQUEST_ID),
        target: fixtures::target(),
        principal: fixtures::principal(),
        mode: LeaseMode::Acquire {},
    });
    rejects_unknown(InputRequest::new(fixtures::input_fields()).unwrap());
    rejects_unknown(ResizeRequest {
        request_id: Uuid::from_u128(REQUEST_ID),
        grant: LeaseGrant::new(fixtures::grant_fields()).unwrap(),
        size: TerminalSize::default(),
    });
    rejects_unknown(TerminalSnapshot::new(fixtures::snapshot_fields(TEST_SEQUENCE)).unwrap());
    rejects_unknown(TerminalHistory::new(fixtures::history_fields(TEST_SEQUENCE)).unwrap());
    rejects_unknown(OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE)).unwrap());
    rejects_unknown(OutputCursor {
        target: fixtures::target(),
        after_sequence: TEST_SEQUENCE,
    });
    rejects_unknown(AttachRequest::new(fixtures::target(), None).unwrap());
    rejects_unknown(AttachRequest::history(fixtures::target(), None).unwrap());
    rejects_unknown(AttachResult::new(fixtures::replay_fields()).unwrap());
    rejects_unknown(AttachResult::new(fixtures::history_result_fields()).unwrap());
}

#[test]
fn tagged_empty_actions_reject_injected_payload_fields() {
    for action in [
        TerminalAction::Resume {},
        TerminalAction::Recreate {},
        TerminalAction::Archive {},
        TerminalAction::Terminate { force: false },
    ] {
        rejects_unknown(action);
    }
    for mode in [
        LeaseMode::Acquire {},
        LeaseMode::TakeControl {
            observed_generation: FIRST_LEASE_GENERATION,
        },
    ] {
        rejects_unknown(mode);
    }
}

#[test]
fn wire_decoding_runs_metadata_and_payload_validation() {
    let create = CreateShellRequest::new(fixtures::create_fields()).unwrap();
    reject_field(
        &create,
        field::SCHEMA_VERSION,
        (TERMINAL_PROTOCOL_SCHEMA_VERSION + STEP as u16).into(),
    );
    reject_field(&create, field::CWD, INVALID_TEXT[0].into());
    reject_field(&create, field::NAME, INVALID_TEXT[0].into());
    let summary = TerminalSummary::new(fixtures::summary_fields()).unwrap();
    reject_field(&summary, field::NAME, INVALID_TEXT[0].into());
    reject_field(&summary, field::CWD, INVALID_TEXT[0].into());
    let size = TerminalSize::default();
    reject_field(&size, field::COLS, 0.into());
    reject_field(&size, field::ROWS, (MAX_TERMINAL_ROWS + STEP as u16).into());
    let input = InputRequest::new(fixtures::input_fields()).unwrap();
    reject_field(&input, field::DATA, serde_json::json!([]));
    let output = OutputChunk::new(fixtures::chunk_fields(TEST_SEQUENCE)).unwrap();
    reject_field(&output, field::SEQUENCE, 0.into());
    reject_field(&output, field::DATA, serde_json::json!([]));
    let result = AttachResult::new(fixtures::replay_fields()).unwrap();
    reject_field(&result, field::NEXT_SEQUENCE, 0.into());
    reject_field(&result, field::RESYNCHRONIZED, true.into());
    let cursor = OutputCursor {
        target: fixtures::target(),
        after_sequence: TEST_SEQUENCE,
    };
    reject_field(&cursor, field::AFTER_SEQUENCE, u64::MAX.into());
}

#[test]
fn wire_lease_validation_cannot_be_bypassed_by_a_valid_secret() {
    let grant = LeaseGrant::new(fixtures::grant_fields()).unwrap();
    for field in [
        field::GENERATION,
        field::TTL_MS,
        field::RENEW_AFTER_MS,
        field::EXPIRES_AT_MS,
    ] {
        reject_field(&grant, field, 0.into());
    }
    reject_field(&grant, field::CAPABILITY, NIL_ID.into());
    let principal = fixtures::principal();
    for field in [field::CLIENT_ID, field::CONTROLLER_INSTANCE_ID] {
        reject_field(&principal, field, NIL_ID.into());
    }
}
