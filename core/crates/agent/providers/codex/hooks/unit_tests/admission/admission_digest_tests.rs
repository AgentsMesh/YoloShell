use super::{CodexExecutableAttestation, CodexExecutableKind};
use crate::sha256_digest::Sha256Digest;
use sha2::{Digest, Sha256};

#[path = "../../test_support/admission.rs"]
mod support;

const DIGEST_DOMAIN: &str = "yolo.codex-admission.executable.v1";
const SHA256_BYTES: usize = 32;
const CHUNK_BOUNDARY_BYTES: usize = 64 * 1_024;
const BOUNDARY_OFFSET: usize = 1;
const CONTENT_BYTE: u8 = 0xa5;
const REPLACEMENT_BYTE: u8 = 0x5a;

#[test]
fn accelerated_digest_preserves_durable_sha256_across_read_boundaries() {
    let fixture = support::Fixture::new();
    for length in [
        BOUNDARY_OFFSET,
        CHUNK_BOUNDARY_BYTES - BOUNDARY_OFFSET,
        CHUNK_BOUNDARY_BYTES,
        CHUNK_BOUNDARY_BYTES + BOUNDARY_OFFSET,
    ] {
        let contents = vec![CONTENT_BYTE; length];
        fixture.replace_hook(&contents);
        let attestation = CodexExecutableAttestation::capture(
            &fixture.request.hook_executable,
            CodexExecutableKind::Runtime,
        )
        .unwrap();
        let raw: [u8; SHA256_BYTES] = Sha256::digest(&contents).into();
        assert_eq!(
            attestation.content_digest(),
            Sha256Digest::capture(&(DIGEST_DOMAIN, raw)).unwrap()
        );
        attestation
            .verify_current(CodexExecutableKind::Runtime)
            .unwrap();
    }
}

#[test]
fn accelerated_digest_rejects_in_place_same_length_rewrites() {
    let fixture = support::Fixture::new();
    let contents = vec![CONTENT_BYTE; CHUNK_BOUNDARY_BYTES + BOUNDARY_OFFSET];
    fixture.replace_hook(&contents);
    let original = CodexExecutableAttestation::capture(
        &fixture.request.hook_executable,
        CodexExecutableKind::Runtime,
    )
    .unwrap();
    std::fs::write(
        &fixture.request.hook_executable,
        vec![REPLACEMENT_BYTE; contents.len()],
    )
    .unwrap();
    assert!(
        original
            .verify_current(CodexExecutableKind::Runtime)
            .is_err()
    );
    let replaced = CodexExecutableAttestation::capture(
        &fixture.request.hook_executable,
        CodexExecutableKind::Runtime,
    )
    .unwrap();
    assert_ne!(original.content_digest(), replaced.content_digest());
}
