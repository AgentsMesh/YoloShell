use super::support::*;
use crate::{AccessRegistry, RegistryError};
use yolo_hub_client_access_protocol::AccessError;
use yolo_hub_routing::Right;

#[test]
fn admission_exposes_only_the_exact_independent_client_grant() {
    let configuration = configuration();
    let registry = AccessRegistry::new(configuration.clone()).unwrap();
    let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    assert_eq!(registry.current().unwrap(), configuration);
    assert_eq!(admission.grant(), &configuration.clients[FIRST_CLIENT]);
    assert_eq!(admission.entry_hub(), configuration.entry_hub);
    assert_eq!(admission.revision(), configuration.revision);
    assert!(admission.is_current(NOW_MS));
    assert!(
        admission
            .grant()
            .allows(configuration.entry_hub, &rights(Right::TerminalRead))
    );
    assert!(
        !admission
            .grant()
            .allows(configuration.entry_hub, &rights(Right::Observe))
    );
    assert!(
        !admission
            .grant()
            .allows(configuration.entry_hub, &rights(Right::TerminalWrite))
    );
    assert!(matches!(
        registry.admit_verified_key(&OTHER_KEY, NOW_MS),
        Err(RegistryError::Unauthorized)
    ));
}

#[test]
fn both_new_and_existing_admission_expire_at_the_exact_boundary() {
    let registry = AccessRegistry::new(configuration()).unwrap();
    let admission = registry
        .admit_verified_key(&CLIENT_KEY, BEFORE_EXPIRY_MS)
        .unwrap();
    assert!(admission.is_current(BEFORE_EXPIRY_MS));
    for now in [EXPIRES_AT_MS, u64::MAX] {
        assert!(!admission.is_current(now));
        assert!(matches!(
            registry.admit_verified_key(&CLIENT_KEY, now),
            Err(RegistryError::Access(AccessError::Expired))
        ));
    }
}

#[test]
fn invalid_configuration_cannot_create_or_replace_a_registry() {
    let original = configuration();
    let mut invalid = original.clone();
    invalid.clients.push(invalid.clients[FIRST_CLIENT].clone());
    assert!(matches!(
        AccessRegistry::new(invalid.clone()),
        Err(RegistryError::Access(AccessError::DuplicateClient))
    ));
    let registry = AccessRegistry::new(original.clone()).unwrap();
    let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    invalid.revision += REVISION_STEP;
    assert!(registry.replace(original.revision, invalid).is_err());
    assert_eq!(registry.current().unwrap(), original);
    assert!(admission.is_current(NOW_MS));
}

#[test]
fn close_is_idempotent_and_cannot_be_reopened_by_replacement() {
    let original = configuration();
    let registry = AccessRegistry::new(original.clone()).unwrap();
    let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    registry.close();
    registry.close();
    assert!(!admission.is_current(NOW_MS));
    assert_eq!(registry.current(), Err(RegistryError::Closed));
    assert_eq!(
        registry.replace(original.revision, next(&original)),
        Err(RegistryError::Closed)
    );
    assert!(matches!(
        registry.admit_verified_key(&CLIENT_KEY, NOW_MS),
        Err(RegistryError::Closed)
    ));
}

#[test]
fn errors_describe_policy_failures_without_credentials() {
    for error in [
        RegistryError::Access(AccessError::Expired),
        RegistryError::Closed,
        RegistryError::RevisionConflict,
        RegistryError::WrongEntry,
        RegistryError::RevisionExhausted,
        RegistryError::Unavailable,
        RegistryError::Unauthorized,
    ] {
        assert!(!error.to_string().is_empty());
    }
}
