use super::*;
use crate::remote_broker::credential_binding::{
    CREDENTIAL_BINDING_MISMATCH, CredentialBindingFile, ordered_identity_paths_sha256,
};
use serde_json::json;
use std::{io, path::PathBuf};

const FIRST_SECRET: &str = "first-secret";
const SECOND_SECRET: &str = "second-secret";
const HOST_A: &str = "host-a";
const HOST_B: &str = "host-b";
const NONCE_A: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const NONCE_B: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const REVISION_A: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const REVISION_B: &str = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
const IDENTITY_A: &str = "/tmp/identity-a";
const IDENTITY_B: &str = "/tmp/identity-b";

fn encoded(
    nonce: &str,
    host: &str,
    revision: Option<&str>,
    paths: &[PathBuf],
    passphrases: Vec<Option<&str>>,
) -> Vec<u8> {
    yolo_capnp_codec::encode(&json!({
        "version": WIRE_VERSION,
        (BINDING_NONCE_FIELD): nonce,
        (HOST_ID_FIELD): host,
        (CREDENTIAL_REVISION_FIELD): revision,
        (IDENTITY_PATHS_DIGEST_FIELD): ordered_identity_paths_sha256(paths).unwrap(),
        "passphrases": passphrases,
    }))
    .unwrap()
}

fn paths() -> Vec<PathBuf> {
    [IDENTITY_A, IDENTITY_B].map(PathBuf::from).to_vec()
}

fn configured(nonce: &str, paths: &[PathBuf]) -> CredentialBindingFile {
    CredentialBindingFile {
        nonce: nonce.into(),
        identity_paths_sha256: ordered_identity_paths_sha256(paths).unwrap(),
    }
}

#[test]
fn private_input_round_trips_and_rebinds_without_exposing_secrets() {
    let paths = paths();
    let values = IdentityPassphrases::read_checked(
        encoded(
            NONCE_A,
            HOST_A,
            Some(REVISION_A),
            &paths,
            vec![Some(FIRST_SECRET), None],
        )
        .as_slice(),
        false,
    )
    .unwrap();
    values
        .validate_source(
            HOST_A,
            Some(REVISION_A),
            &paths,
            Some(&configured(NONCE_A, &paths)),
        )
        .unwrap();
    assert_eq!(values.get(0).unwrap().as_str(), FIRST_SECRET);
    assert!(values.get(1).is_none());
    let rebound = IdentityPassphrases::read_checked(
        values.encode_rebound(Some(NONCE_B)).unwrap().as_slice(),
        false,
    )
    .unwrap();
    rebound
        .validate_snapshot(
            HOST_A,
            Some(REVISION_A),
            Some(&configured(NONCE_B, &paths)),
            true,
        )
        .unwrap();
    assert_eq!(rebound.get(0).unwrap().as_str(), FIRST_SECRET);
}

#[test]
fn source_and_snapshot_reject_every_cross_launch_binding_swap() {
    let paths = paths();
    let values = IdentityPassphrases::read_checked(
        encoded(NONCE_A, HOST_A, Some(REVISION_A), &paths, vec![None, None]).as_slice(),
        false,
    )
    .unwrap();
    let reversed = paths.iter().cloned().rev().collect::<Vec<_>>();
    let cases = [
        values.validate_source(
            HOST_A,
            Some(REVISION_A),
            &paths,
            Some(&configured(NONCE_B, &paths)),
        ),
        values.validate_source(
            HOST_B,
            Some(REVISION_A),
            &paths,
            Some(&configured(NONCE_A, &paths)),
        ),
        values.validate_source(
            HOST_A,
            Some(REVISION_B),
            &paths,
            Some(&configured(NONCE_A, &paths)),
        ),
        values.validate_source(
            HOST_A,
            Some(REVISION_A),
            &reversed,
            Some(&configured(NONCE_A, &paths)),
        ),
        values.validate_snapshot(
            HOST_A,
            Some(REVISION_A),
            Some(&configured(NONCE_B, &paths)),
            true,
        ),
    ];
    for result in cases {
        let error = result.unwrap_err();
        assert!(error.to_string().contains(CREDENTIAL_BINDING_MISMATCH));
    }
}

#[test]
fn absent_input_supports_unencrypted_identities_but_provided_count_is_exact() {
    let values = IdentityPassphrases::none();
    values.validate_count(MAX_IDENTITY_FILES).unwrap();
    assert!(values.get(0).is_none());
    let paths = Vec::new();
    let provided = IdentityPassphrases::read_checked(
        encoded(NONCE_A, HOST_A, None, &paths, Vec::new()).as_slice(),
        false,
    )
    .unwrap();
    provided.validate_count(0).unwrap();
    assert!(provided.validate_count(1).is_err());
}

#[test]
fn input_rejects_terminal_shape_versions_binding_and_secret_limits() {
    let valid = encoded(NONCE_A, HOST_A, None, &[], Vec::new());
    assert!(IdentityPassphrases::read_checked(valid.as_slice(), true).is_err());
    for mutation in [
        json!({"version": 1, "passphrases": []}),
        json!({"version": WIRE_VERSION, "passphrases": [], "extra": true}),
    ] {
        let bytes = yolo_capnp_codec::encode(&mutation).unwrap();
        assert!(IdentityPassphrases::read_checked(bytes.as_slice(), false).is_err());
    }
    let oversized = "x".repeat(MAX_PASSPHRASE_BYTES + 1);
    let bytes = encoded(NONCE_A, HOST_A, None, &[], vec![Some(oversized.as_str())]);
    assert!(IdentityPassphrases::read_checked(bytes.as_slice(), false).is_err());
    let bytes = encoded(NONCE_A, HOST_A, None, &[], vec![Some("")]);
    assert!(IdentityPassphrases::read_checked(bytes.as_slice(), false).is_err());
    assert!(
        IdentityPassphrases::read_checked(&vec![b'x'; MAX_INPUT_BYTES + 1][..], false).is_err()
    );
}

#[test]
fn reader_failure_and_binding_errors_are_secret_redacted() {
    let error = IdentityPassphrases::read_checked(FailingReader, false)
        .err()
        .unwrap()
        .to_string();
    assert!(!error.contains(FIRST_SECRET));
    let values = IdentityPassphrases::read_checked(
        encoded(NONCE_A, HOST_A, None, &[], vec![Some(SECOND_SECRET)]).as_slice(),
        false,
    )
    .unwrap();
    assert!(
        !values
            .validate_count(0)
            .unwrap_err()
            .to_string()
            .contains(SECOND_SECRET)
    );
}

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("injected private input failure"))
    }
}
