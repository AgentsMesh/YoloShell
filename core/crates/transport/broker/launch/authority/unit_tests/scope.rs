use super::{constants::*, fixture::Fixture};
use crate::*;

#[tokio::test]
async fn cloned_scope_dto_does_not_authorize_an_unbound_child() {
    let fixture = Fixture::new();
    let (issuer, _fd) = issue(
        fixture.owner.clone(),
        fixture.scope.clone(),
        None,
        || Ok(()),
    )
    .unwrap();
    assert!(issuer.serve().await.is_err());
}

#[test]
fn source_replacement_even_with_identical_bytes_revokes_the_scope() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.scope.config_sha256(),
        yolo_crypto::sha256_hex(SOURCE)
    );
    fixture.scope.verify().unwrap();
    let source = &fixture.scope.config_source;
    std::fs::rename(source, fixture.root.path().join("retained-config")).unwrap();
    std::fs::write(source, SOURCE).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(source, std::fs::Permissions::from_mode(PRIVATE_MODE)).unwrap();
    assert!(fixture.scope.verify().is_err());
}

#[test]
fn config_content_and_broker_namespace_cannot_change_under_a_permit() {
    let fixture = Fixture::new();
    std::fs::write(&fixture.scope.config_source, REPLACED).unwrap();
    assert!(fixture.scope.verify().is_err());
    let fixture = Fixture::new();
    std::fs::rename(
        &fixture.scope.broker_state,
        fixture.root.path().join("retained-broker"),
    )
    .unwrap();
    yolo_remote_broker::BrokerPaths::new(&fixture.scope.broker_state).unwrap();
    assert!(fixture.scope.verify().is_err());
}

#[tokio::test]
async fn issuer_rechecks_owner_callback_before_sending_any_authority() {
    let fixture = Fixture::new();
    assert!(
        issue(
            fixture.owner.clone(),
            fixture.scope.clone(),
            None,
            || anyhow::bail!("profile revoked")
        )
        .is_err()
    );
}

#[tokio::test]
async fn invalid_private_credential_errors_do_not_retain_the_source_secret() {
    let fixture = Fixture::new();
    let bytes = zeroize::Zeroizing::new(serde_json::to_vec(EXPECTED_SECRET).unwrap());
    let error = issue(fixture.owner.clone(), fixture.scope, Some(bytes), || Ok(()))
        .err()
        .unwrap();
    assert!(!format!("{error:#}").contains(EXPECTED_SECRET));
}
