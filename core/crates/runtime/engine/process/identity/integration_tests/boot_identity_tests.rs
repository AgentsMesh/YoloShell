use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const CACHE_THREADS: usize = 16;
const CACHED_RESOLUTION_ATTEMPTS: usize = 1;
const TRANSIENT_RESOLUTION_ATTEMPTS: usize = 2;
const TEST_KNOWN_BOOT_ID: &str = "test-known-boot";
const TEST_OTHER_BOOT_ID: &str = "test-other-boot";
const TEST_UNKNOWN_BOOT_ID: &str = "unknown:test-transient";
const UNEXPECTED_FALLBACK_DETAIL: &str = "known boot identity must bypass the fallback";
const UNEXPECTED_RESOLVER_DETAIL: &str = "known boot identity must bypass the resolver";

#[test]
fn unknown_resolution_is_not_cached_and_a_known_retry_is_stable() {
    let cache = BootIdCache::new();
    let attempts = AtomicUsize::new(0);
    let first = cache.resolve(
        || {
            attempts.fetch_add(1, Ordering::SeqCst);
            None
        },
        || TEST_UNKNOWN_BOOT_ID.into(),
    );
    let second = cache.resolve(
        || {
            attempts.fetch_add(1, Ordering::SeqCst);
            Some(TEST_KNOWN_BOOT_ID.into())
        },
        || TEST_UNKNOWN_BOOT_ID.into(),
    );
    let third = cache.resolve(
        || panic!("{UNEXPECTED_RESOLVER_DETAIL}"),
        || panic!("{UNEXPECTED_FALLBACK_DETAIL}"),
    );

    assert_eq!(first, TEST_UNKNOWN_BOOT_ID);
    assert_eq!(second, TEST_KNOWN_BOOT_ID);
    assert_eq!(third, TEST_KNOWN_BOOT_ID);
    assert_eq!(
        attempts.load(Ordering::SeqCst),
        TRANSIENT_RESOLUTION_ATTEMPTS
    );
}

#[test]
fn resolver_output_must_itself_be_a_known_identity_before_caching() {
    let cache = BootIdCache::new();
    let first = cache.resolve(
        || Some(TEST_UNKNOWN_BOOT_ID.into()),
        || TEST_UNKNOWN_BOOT_ID.into(),
    );
    let second = cache.resolve(
        || Some(TEST_KNOWN_BOOT_ID.into()),
        || TEST_UNKNOWN_BOOT_ID.into(),
    );

    assert_eq!(first, TEST_UNKNOWN_BOOT_ID);
    assert_eq!(second, TEST_KNOWN_BOOT_ID);
}

#[test]
fn concurrent_known_resolution_is_cached_once() {
    let cache = Arc::new(BootIdCache::new());
    let attempts = Arc::new(AtomicUsize::new(0));
    let handles = (0..CACHE_THREADS)
        .map(|_| {
            let cache = Arc::clone(&cache);
            let attempts = Arc::clone(&attempts);
            std::thread::spawn(move || {
                cache.resolve(
                    || {
                        attempts.fetch_add(1, Ordering::SeqCst);
                        Some(TEST_KNOWN_BOOT_ID.into())
                    },
                    || TEST_UNKNOWN_BOOT_ID.into(),
                )
            })
        })
        .collect::<Vec<_>>();
    let values = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    assert!(values.iter().all(|value| value == TEST_KNOWN_BOOT_ID));
    assert_eq!(attempts.load(Ordering::SeqCst), CACHED_RESOLUTION_ATTEMPTS);
}

#[test]
fn generic_known_and_unknown_relations_remain_typed() {
    assert_eq!(
        boot_identity_relation(TEST_KNOWN_BOOT_ID, TEST_KNOWN_BOOT_ID),
        BootIdentityRelation::Same
    );
    assert_eq!(
        boot_identity_relation(TEST_KNOWN_BOOT_ID, TEST_OTHER_BOOT_ID),
        BootIdentityRelation::Different
    );
    assert_eq!(
        boot_identity_relation(TEST_UNKNOWN_BOOT_ID, TEST_KNOWN_BOOT_ID),
        BootIdentityRelation::Unknown
    );
}

#[cfg(target_os = "macos")]
#[test]
fn runtime_and_custody_identities_match_the_current_boot() {
    const NATIVE_IDENTITY_REQUIRED: &str = "native macOS boot-session identity must be available";
    let session = macos::resolve().expect(NATIVE_IDENTITY_REQUIRED);
    assert_eq!(boot_id(), session);
    let custody = digest::BootIdentity::from_value(&session);
    assert!(custody.known());
    assert_eq!(custody.relation_to_current(), BootIdentityRelation::Same);
}

#[cfg(target_os = "macos")]
#[test]
fn malformed_macos_identity_is_never_treated_as_known() {
    const MALFORMED_MACOS_BOOT_ID: &str = "macos:not-a-boot-identity";

    assert_eq!(
        boot_identity_relation(MALFORMED_MACOS_BOOT_ID, MALFORMED_MACOS_BOOT_ID),
        BootIdentityRelation::Unknown
    );
    assert!(!digest::BootIdentity::from_value(MALFORMED_MACOS_BOOT_ID).known());
}
