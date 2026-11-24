use super::support::*;
use crate::{AccessRegistry, RegistryError};
use yolo_hub_identity::HubId;

#[test]
fn replacement_requires_exact_cas_successor_and_entry_hub() {
    let original = configuration();
    let registry = AccessRegistry::new(original.clone()).unwrap();
    let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    assert_eq!(
        registry.replace(original.revision + REVISION_STEP, next(&original)),
        Err(RegistryError::RevisionConflict)
    );
    assert_eq!(
        registry.replace(original.revision, original.clone()),
        Err(RegistryError::RevisionConflict)
    );
    let mut skipped = original.clone();
    skipped.revision += REVISION_GAP;
    assert_eq!(
        registry.replace(original.revision, skipped),
        Err(RegistryError::RevisionConflict)
    );
    let mut foreign = next(&original);
    foreign.entry_hub = HubId::new();
    assert_eq!(
        registry.replace(original.revision, foreign),
        Err(RegistryError::WrongEntry)
    );
    assert_eq!(registry.current().unwrap(), original);
    assert!(admission.is_current(NOW_MS));
    registry
        .replace(original.revision, next(&original))
        .unwrap();
    assert!(!admission.is_current(NOW_MS));
    assert!(
        registry
            .admit_verified_key(&CLIENT_KEY, NOW_MS)
            .unwrap()
            .is_current(NOW_MS)
    );
}

#[test]
fn exhausted_revision_never_wraps_or_restores_old_authority() {
    let mut original = configuration();
    original.revision = u64::MAX;
    let registry = AccessRegistry::new(original.clone()).unwrap();
    let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
    assert_eq!(
        registry.replace(original.revision, original.clone()),
        Err(RegistryError::RevisionExhausted)
    );
    assert!(admission.is_current(NOW_MS));
    assert_eq!(registry.current().unwrap(), original);
}

#[test]
fn removing_or_rotating_a_key_revokes_every_prior_admission() {
    for remove in [false, true] {
        let original = configuration();
        let registry = AccessRegistry::new(original.clone()).unwrap();
        let first = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
        let second = first.clone();
        let mut changed = next(&original);
        if remove {
            changed.clients.clear();
        } else {
            changed.clients[FIRST_CLIENT].identity.public_key = OTHER_KEY;
        }
        registry.replace(original.revision, changed).unwrap();
        assert!(!first.is_current(NOW_MS));
        assert!(!second.is_current(NOW_MS));
        assert!(matches!(
            registry.admit_verified_key(&CLIENT_KEY, NOW_MS),
            Err(RegistryError::Unauthorized)
        ));
        assert_eq!(
            registry.admit_verified_key(&OTHER_KEY, NOW_MS).is_ok(),
            !remove
        );
    }
}
