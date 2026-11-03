use crate::{
    configuration::{Configuration, write_private},
    constants::*,
};
use std::{
    ffi::OsString,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};
use yolo_web_gateway::{MAX_AUTH_BYTES, MAX_PROFILE_BYTES};

async fn rejected(arguments: Vec<OsString>) {
    let result = tokio::task::spawn_blocking(move || yolo_web_cli::run_from(arguments))
        .await
        .unwrap();
    assert!(result.is_err());
    assert!(!format!("{:#}", result.unwrap_err()).contains(TOKEN));
}

#[tokio::test]
async fn invalid_profiles_keys_and_tokens_are_rejected_from_real_private_files() {
    for (member, bytes) in [
        (PROFILE_OPTION, b"invalid JSON".as_slice()),
        (KEY_OPTION, b"invalid private key".as_slice()),
        (KEY_OPTION, &[u8::MAX]),
        (TOKEN_OPTION, b"too short".as_slice()),
        (TOKEN_OPTION, &[u8::MAX]),
    ] {
        let configuration = Configuration::new();
        let path = match member {
            PROFILE_OPTION => &configuration.profile,
            KEY_OPTION => &configuration.key,
            _ => &configuration.token,
        };
        write_private(path, bytes);
        rejected(configuration.arguments()).await;
    }
    let configuration = Configuration::new();
    configuration.corrupt_profile(HUB_FIELD, serde_json::Value::Null);
    rejected(configuration.arguments()).await;
    let configuration = Configuration::new();
    let mut identity = configuration.identity;
    identity.public_key = WRONG_KEY_BYTES;
    configuration.corrupt_profile(CLIENT_FIELD, serde_json::to_value(identity).unwrap());
    rejected(configuration.arguments()).await;
}

#[tokio::test]
async fn filesystem_permissions_symlinks_and_bounds_are_enforced_before_binding() {
    let configuration = Configuration::new();
    fs::set_permissions(
        configuration.root.path(),
        fs::Permissions::from_mode(PUBLIC_MODE),
    )
    .unwrap();
    rejected(configuration.arguments()).await;
    fs::set_permissions(
        configuration.root.path(),
        fs::Permissions::from_mode(DIRECTORY_MODE),
    )
    .unwrap();
    fs::set_permissions(
        &configuration.token,
        fs::Permissions::from_mode(PUBLIC_MODE),
    )
    .unwrap();
    rejected(configuration.arguments()).await;
    let configuration = Configuration::new();
    fs::remove_file(&configuration.token).unwrap();
    symlink(&configuration.key, &configuration.token).unwrap();
    rejected(configuration.arguments()).await;
    for (member, limit) in [
        (PROFILE_OPTION, MAX_PROFILE_BYTES),
        (TOKEN_OPTION, MAX_AUTH_BYTES as u64),
    ] {
        let configuration = Configuration::new();
        let path = if member == PROFILE_OPTION {
            &configuration.profile
        } else {
            &configuration.token
        };
        write_private(path, &vec![b'x'; limit as usize + 1]);
        rejected(configuration.arguments()).await;
    }
}

#[tokio::test]
async fn invalid_listeners_origins_and_limits_do_not_start_a_gateway() {
    for (option, value) in [
        (LISTEN_OPTION, "0.0.0.0:8080"),
        (ORIGIN_OPTION, "null"),
        (CAPACITY_OPTION, "0"),
        (LIFETIME_OPTION, "0"),
    ] {
        let configuration = Configuration::new();
        let mut arguments = configuration.arguments();
        arguments.extend([option.into(), value.into()]);
        rejected(arguments).await;
    }
    let configuration = Configuration::new();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut arguments = configuration.arguments();
    arguments.extend([
        LISTEN_OPTION.into(),
        listener.local_addr().unwrap().to_string().into(),
    ]);
    rejected(arguments).await;
}
