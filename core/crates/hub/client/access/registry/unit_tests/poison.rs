use super::AccessRegistry;
use crate::{RegistryError, unit_tests::support::*};

#[tokio::test]
async fn poisoned_policy_owner_fails_closed_and_close_still_revokes() {
    for close_first in [false, true] {
        let original = configuration();
        let registry = AccessRegistry::new(original.clone()).unwrap();
        let admission = registry.admit_verified_key(&CLIENT_KEY, NOW_MS).unwrap();
        let owner = registry.clone();
        let poisoned = std::thread::spawn(move || {
            let _held = owner.state().unwrap();
            panic!("{POISON_MESSAGE}");
        });
        assert!(poisoned.join().is_err());
        assert!(!admission.is_current(NOW_MS));
        tokio::time::timeout(WAKE_TIMEOUT, admission.revoked())
            .await
            .unwrap();
        if close_first {
            registry.close();
        }
        assert_eq!(registry.current(), Err(RegistryError::Unavailable));
        assert!(!admission.is_current(NOW_MS));
        assert_eq!(
            registry.replace(original.revision, next(&original)),
            Err(RegistryError::Unavailable)
        );
        assert!(matches!(
            registry.admit_verified_key(&CLIENT_KEY, NOW_MS),
            Err(RegistryError::Unavailable)
        ));
    }
}
