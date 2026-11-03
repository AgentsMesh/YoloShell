use super::constants::snapshot_fields as fields;
use super::test_support::{CLAUDE_VERSION, CODEX_VERSION, envelope, identity, started};
use super::*;
use serde_json::{Value, from_value, to_value};

const REJECTED_SEQUENCE: u64 = FIRST_SEQUENCE + 2;
const OBSERVED_START_DELTA: u64 = 1;
const TAMPER_DELTA: u64 = 1;
const TAMPERED_DIGEST_BYTE: u8 = 0xa5;

#[derive(serde::Serialize)]
struct CompactDigest(
    #[serde(with = "crate::sha256_digest::compact")] crate::sha256_digest::Sha256Digest,
);

fn hook_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let rejected = envelope(
        &expected,
        CODEX_VERSION,
        REJECTED_SEQUENCE,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(reducer.apply(rejected), Err(HookViolation::SequenceGap));
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

fn process_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let observed = ProcessIdentity::new(
        expected.process.pid,
        expected.process.start_time + OBSERVED_START_DELTA,
    );
    assert_eq!(
        reducer.process_exit_with_identity(observed),
        Err(HookViolation::ProcessMismatch)
    );
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

fn revocation(value: &mut Value) -> &mut Value {
    &mut value[fields::LIFECYCLE][fields::REVOCATION]
}

fn predecessor(value: &mut Value) -> &mut Value {
    &mut revocation(value)[fields::PREDECESSOR]
}

fn assert_invalid(value: Value, expected: HookIdentity) {
    let snapshot = from_value::<ReducerSnapshot>(value).unwrap();
    assert!(Reducer::restore(snapshot, expected, HookProviderKind::Codex, CODEX_VERSION).is_err());
}

fn tampered_digest() -> Value {
    to_value(CompactDigest(crate::sha256_digest::Sha256Digest::repeated(
        TAMPERED_DIGEST_BYTE,
    )))
    .unwrap()
}

#[test]
fn predecessor_provider_and_version_must_match_top_level_binding() {
    let (expected, mut provider) = hook_value();
    predecessor(&mut provider)[fields::BINDING][fields::PROVIDER][fields::KIND] =
        Value::String(provider_names::CLAUDE.to_owned());
    assert_invalid(provider, expected);

    let (expected, mut version) = hook_value();
    predecessor(&mut version)[fields::BINDING][fields::PROVIDER_VERSION] =
        to_value(CLAUDE_VERSION).unwrap();
    assert_invalid(version, expected);
}

#[test]
fn top_level_state_must_be_the_canonical_replay_output() {
    let (expected, mut value) = hook_value();
    value[fields::LIFECYCLE][fields::PROCESS] = to_value(ProcessState::Alive).unwrap();
    assert_invalid(value, expected);
}

#[test]
fn predecessor_digest_rejects_direct_tampering() {
    let (expected, mut value) = hook_value();
    revocation(&mut value)[fields::PREDECESSOR_SHA256] = tampered_digest();
    assert_invalid(value, expected);
}

#[test]
fn another_valid_process_mismatch_cannot_replace_the_observation() {
    let (expected, mut value) = process_value();
    let start_time = revocation(&mut value)[fields::START_TIME].as_u64().unwrap();
    revocation(&mut value)[fields::START_TIME] = Value::from(start_time + TAMPER_DELTA);
    assert_invalid(value, expected);
}

#[test]
fn process_observation_digest_rejects_direct_tampering() {
    let (expected, mut value) = process_value();
    revocation(&mut value)[fields::OBSERVATION_SHA256] = tampered_digest();
    assert_invalid(value, expected);
}
