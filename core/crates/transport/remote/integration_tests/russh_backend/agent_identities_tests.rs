use super::*;
use russh::keys::ssh_encoding::Encode;
use std::time::Duration;

const TEST_AGENT_STREAM_BYTES: usize = 64;
const NO_IDENTITIES: usize = 0;
const SINGLE_IDENTITY: usize = 1;
const DECLARED_KEY_BYTES: u32 = 1;
const TRAILING_RESPONSE_BYTE: u8 = 0xff;
const CERTIFICATE_SERIAL: u64 = 7;
const CERTIFICATE_VALID_AFTER: Duration = Duration::from_secs(1);
const CERTIFICATE_VALID_BEFORE: Duration = Duration::from_secs(2);
const CERTIFICATE_KEY_ID: &str = "agent-certificate";
const CERTIFICATE_PRINCIPAL: &str = "remote-user";
const OTHER_EFFECTIVE_UID_OFFSET: libc::uid_t = 1;

fn encode_string(value: &[u8], output: &mut Vec<u8>) {
    output.extend_from_slice(&u32::try_from(value.len()).unwrap().to_be_bytes());
    output.extend_from_slice(value);
}

fn identities_response(count: usize, key_blob: &[u8]) -> Vec<u8> {
    let mut response = vec![SSH_AGENT_IDENTITIES_ANSWER_MESSAGE];
    response.extend_from_slice(&u32::try_from(count).unwrap().to_be_bytes());
    for _ in 0..count {
        encode_string(key_blob, &mut response);
        encode_string(&[], &mut response);
    }
    response
}

fn public_key_blob() -> Vec<u8> {
    let mut rng = russh::keys::key::safe_rng();
    let key = russh::keys::PrivateKey::random(&mut rng, russh::keys::Algorithm::Ed25519).unwrap();
    let mut blob = Vec::new();
    key.public_key().key_data().encode(&mut blob).unwrap();
    blob
}

fn certificate_blob() -> Vec<u8> {
    let mut rng = russh::keys::key::safe_rng();
    let ca = russh::keys::PrivateKey::random(&mut rng, russh::keys::Algorithm::Ed25519).unwrap();
    let subject =
        russh::keys::PrivateKey::random(&mut rng, russh::keys::Algorithm::Ed25519).unwrap();
    let mut builder = russh::keys::ssh_key::certificate::Builder::new_with_random_nonce(
        &mut rng,
        subject.public_key(),
        CERTIFICATE_VALID_AFTER.as_secs(),
        CERTIFICATE_VALID_BEFORE.as_secs(),
    )
    .unwrap();
    builder.serial(CERTIFICATE_SERIAL).unwrap();
    builder.key_id(CERTIFICATE_KEY_ID).unwrap();
    builder
        .cert_type(russh::keys::ssh_key::certificate::CertType::User)
        .unwrap();
    builder.valid_principal(CERTIFICATE_PRINCIPAL).unwrap();
    let mut blob = Vec::new();
    builder.sign(&ca).unwrap().encode(&mut blob).unwrap();
    blob
}

#[test]
fn identity_count_is_checked_before_any_identity_decode() {
    let response = identities_response(MAX_SSH_AGENT_IDENTITIES + 1, &[]);
    let error = decode_identities(&response).unwrap_err();
    assert!(error.to_string().contains(SSH_AGENT_IDENTITY_LIMIT_CONTEXT));
}

#[test]
fn exact_identity_limit_decodes_without_post_allocation_truncation() {
    let key_blob = public_key_blob();
    let identities =
        decode_identities(&identities_response(MAX_SSH_AGENT_IDENTITIES, &key_blob)).unwrap();
    assert_eq!(identities.len(), MAX_SSH_AGENT_IDENTITIES);
}

#[test]
fn malformed_identity_frames_fail_closed() {
    let mut trailing = identities_response(NO_IDENTITIES, &[]);
    trailing.push(TRAILING_RESPONSE_BYTE);
    let mut truncated_key = vec![SSH_AGENT_IDENTITIES_ANSWER_MESSAGE];
    truncated_key.extend_from_slice(&u32::try_from(SINGLE_IDENTITY).unwrap().to_be_bytes());
    truncated_key.extend_from_slice(&DECLARED_KEY_BYTES.to_be_bytes());
    for response in [
        Vec::new(),
        vec![SSH_AGENT_REQUEST_IDENTITIES_MESSAGE],
        vec![SSH_AGENT_IDENTITIES_ANSWER_MESSAGE],
        trailing,
        truncated_key,
    ] {
        assert!(
            decode_identities(&response).is_err(),
            "accepted {response:?}"
        );
    }
}

#[test]
fn certificate_identity_is_preserved_by_the_bounded_decoder() {
    let blob = certificate_blob();

    let identities = decode_identities(&identities_response(SINGLE_IDENTITY, &blob)).unwrap();

    assert!(matches!(
        identities.as_slice(),
        [russh::keys::agent::AgentIdentity::Certificate { certificate, .. }]
            if certificate.serial() == CERTIFICATE_SERIAL
    ));
}

#[test]
fn identity_key_blobs_reject_internal_trailing_bytes() {
    for mut blob in [public_key_blob(), certificate_blob()] {
        blob.push(TRAILING_RESPONSE_BYTE);
        assert!(decode_identities(&identities_response(SINGLE_IDENTITY, &blob)).is_err());
    }
}

#[tokio::test]
async fn oversized_agent_frame_is_rejected_before_body_allocation_or_read() {
    let (client, mut server) = tokio::io::duplex(TEST_AGENT_STREAM_BYTES);
    let task = tokio::spawn(request_bounded(AgentClient::connect(client)));
    let mut request = [0_u8; SSH_AGENT_LENGTH_PREFIX_BYTES + 1];
    server.read_exact(&mut request).await.unwrap();
    let oversized = u32::try_from(MAX_SSH_AGENT_FRAME_BYTES + 1).unwrap();
    server.write_all(&oversized.to_be_bytes()).await.unwrap();
    let error = task.await.unwrap().err().unwrap();
    assert!(error.to_string().contains(SSH_AGENT_PROTOCOL_MESSAGE));
}

#[test]
fn agent_peer_uid_validation_rejects_a_different_effective_user() {
    let expected = libc::uid_t::default();
    validate_peer_uid(expected, expected).unwrap();
    let error =
        validate_peer_uid(expected.wrapping_add(OTHER_EFFECTIVE_UID_OFFSET), expected).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[cfg(target_os = "macos")]
#[path = "agent_identities_ancillary_tests.rs"]
mod ancillary_tests;
