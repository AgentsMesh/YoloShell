use super::super::{
    BROKER_READY_TIMEOUT, BrokerConfig, BrokerPaths, BrokerReconcileGuard,
    IDENTITY_PASSPHRASE_WIRE_VERSION, IdentityPassphrases,
    stage_broker_config_guarded_with_passphrases,
};
use super::config_tests::ConfigFixture;
use crate::remote_broker::constants::{
    BINDING_NONCE_CONFIG_FIELD, CREDENTIAL_BINDING_CONFIG_FIELD,
    IDENTITY_CREDENTIAL_REVISION_CONFIG_FIELD, IDENTITY_PATHS_SHA256_CONFIG_FIELD,
    PRIVATE_FILE_MODE,
};
use crate::remote_broker::credential_binding::ordered_identity_paths_sha256;
use crate::remote_broker::identity_passphrase_constants::*;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey};
use serde_json::json;
use std::fs;
use std::os::unix::fs::PermissionsExt;

const PASSPHRASE: &str = "encrypted identity passphrase";
const HOST_ID: &str = "build-host";
const NONCE: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const SECOND_NONCE: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
const REVISION: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";

#[tokio::test]
async fn encrypted_identity_requires_ordered_input_without_persisting_the_secret() {
    let fixture = ConfigFixture::new();
    encrypt_identity(&fixture, PASSPHRASE);
    let mut value = fixture.value();
    let paths = vec![fixture.identity.clone()];
    let digest = ordered_identity_paths_sha256(&paths).unwrap();
    value[IDENTITY_CREDENTIAL_REVISION_CONFIG_FIELD] = json!(REVISION);
    value[CREDENTIAL_BINDING_CONFIG_FIELD] = json!({
        (BINDING_NONCE_CONFIG_FIELD): NONCE,
        (IDENTITY_PATHS_SHA256_CONFIG_FIELD): digest,
    });
    let config_path = fixture.write(&value);
    assert!(BrokerConfig::load(&config_path).is_err());
    let encoded = yolo_capnp_codec::encode(&json!({
        "version": IDENTITY_PASSPHRASE_WIRE_VERSION,
        (BINDING_NONCE_FIELD): NONCE,
        (HOST_ID_FIELD): HOST_ID,
        (CREDENTIAL_REVISION_FIELD): REVISION,
        (IDENTITY_PATHS_DIGEST_FIELD): digest,
        "passphrases": [PASSPHRASE],
    }))
    .unwrap();
    let passphrases = IdentityPassphrases::read_checked(encoded.as_slice(), false).unwrap();
    assert!(BrokerConfig::load_with_passphrases(&config_path, &passphrases).is_ok());
    let swapped_input = identity_passphrases(SECOND_NONCE, &digest, PASSPHRASE);
    assert!(BrokerConfig::load_with_passphrases(&config_path, &swapped_input).is_err());

    let wrong = IdentityPassphrases::read_checked(
        yolo_capnp_codec::encode(&json!({
            "version": IDENTITY_PASSPHRASE_WIRE_VERSION,
            (BINDING_NONCE_FIELD): NONCE,
            (HOST_ID_FIELD): HOST_ID,
            (CREDENTIAL_REVISION_FIELD): REVISION,
            (IDENTITY_PATHS_DIGEST_FIELD): digest,
            "passphrases": ["wrong"],
        }))
        .unwrap()
        .as_slice(),
        false,
    )
    .unwrap();
    let error = BrokerConfig::load_with_passphrases(&config_path, &wrong)
        .err()
        .unwrap();
    assert!(!format!("{error:#}").contains("wrong"));

    let mismatched = IdentityPassphrases::read_checked(
        yolo_capnp_codec::encode(&json!({
            "version": IDENTITY_PASSPHRASE_WIRE_VERSION,
            (BINDING_NONCE_FIELD): NONCE,
            (HOST_ID_FIELD): HOST_ID,
            (CREDENTIAL_REVISION_FIELD): REVISION,
            (IDENTITY_PATHS_DIGEST_FIELD): digest,
            "passphrases": [null, null],
        }))
        .unwrap()
        .as_slice(),
        false,
    )
    .unwrap();
    assert!(BrokerConfig::load_with_passphrases(&config_path, &mismatched).is_err());
    let mut invalid_revision = value.clone();
    invalid_revision[IDENTITY_CREDENTIAL_REVISION_CONFIG_FIELD] = json!("invalid");
    assert!(BrokerConfig::load(fixture.write(&invalid_revision)).is_err());

    let paths = BrokerPaths::new(fixture.root.path().join("state")).unwrap();
    let guard =
        BrokerReconcileGuard::acquire(&paths, tokio::time::Instant::now() + BROKER_READY_TIMEOUT)
            .await
            .unwrap();
    let staged =
        stage_broker_config_guarded_with_passphrases(&config_path, &paths, &guard, &passphrases)
            .unwrap();
    let snapshot = fs::read(staged.config_path()).unwrap();
    assert!(
        !snapshot
            .windows(PASSPHRASE.len())
            .any(|value| value == PASSPHRASE.as_bytes())
    );

    let mut second = value;
    second[CREDENTIAL_BINDING_CONFIG_FIELD][BINDING_NONCE_CONFIG_FIELD] = json!(SECOND_NONCE);
    let second_path = fixture.write(&second);
    let second_passphrases = identity_passphrases(SECOND_NONCE, &digest, PASSPHRASE);
    let reused = stage_broker_config_guarded_with_passphrases(
        &second_path,
        &paths,
        &guard,
        &second_passphrases,
    )
    .unwrap();
    assert_eq!(reused.config_path(), staged.config_path());
    assert_eq!(reused.revision(), staged.revision());
    assert_eq!(reused.credential_binding_nonce(), Some(NONCE));
    let detached = IdentityPassphrases::read_checked(
        second_passphrases
            .encode_rebound(reused.credential_binding_nonce())
            .unwrap()
            .as_slice(),
        false,
    )
    .unwrap();
    BrokerConfig::load_expected_with_passphrases(
        reused.config_path(),
        reused.revision(),
        &detached,
    )
    .unwrap();
}

fn identity_passphrases(nonce: &str, digest: &str, secret: &str) -> IdentityPassphrases {
    let encoded = yolo_capnp_codec::encode(&json!({
        "version": IDENTITY_PASSPHRASE_WIRE_VERSION,
        (BINDING_NONCE_FIELD): nonce,
        (HOST_ID_FIELD): HOST_ID,
        (CREDENTIAL_REVISION_FIELD): REVISION,
        (IDENTITY_PATHS_DIGEST_FIELD): digest,
        "passphrases": [secret],
    }))
    .unwrap();
    IdentityPassphrases::read_checked(encoded.as_slice(), false).unwrap()
}

fn encrypt_identity(fixture: &ConfigFixture, passphrase: &str) {
    let mut random = russh::keys::key::safe_rng();
    let key = PrivateKey::random(&mut random, Algorithm::Ed25519)
        .unwrap()
        .encrypt(&mut random, passphrase)
        .unwrap();
    key.write_openssh_file(&fixture.identity, LineEnding::LF)
        .unwrap();
    fs::set_permissions(
        &fixture.identity,
        fs::Permissions::from_mode(PRIVATE_FILE_MODE),
    )
    .unwrap();
}
