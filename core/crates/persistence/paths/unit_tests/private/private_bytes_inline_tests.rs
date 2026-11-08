use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIRST_CONTENT: &[u8] = b"first private script\n";
const MINIMUM_GUARD_CALLS: usize = 6;
const REPLACEMENT_CONTENT: &[u8] = b"replacement private script\n";
const TEST_MAX_BYTES: u64 = 128;
const OVERSIZE_CONTENT_BYTES: usize = TEST_MAX_BYTES as usize + 1;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn atomically_installs_and_replaces_bounded_private_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("integration.zsh");
    let guards = AtomicUsize::new(0);
    for content in [FIRST_CONTENT, REPLACEMENT_CONTENT] {
        write_private_bytes_guarded(
            PrivateFileEntry::ensure(&path).unwrap(),
            content,
            TEST_MAX_BYTES,
            || {
                guards.fetch_add(1, Ordering::Relaxed);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), content);
    }
    assert!(guards.load(Ordering::Relaxed) >= MINIMUM_GUARD_CALLS);
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn size_rejection_preserves_the_installed_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("integration.zsh");
    write_private_bytes_guarded(
        PrivateFileEntry::ensure(&path).unwrap(),
        FIRST_CONTENT,
        TEST_MAX_BYTES,
        || Ok(()),
    )
    .unwrap();
    let oversized = vec![b'x'; OVERSIZE_CONTENT_BYTES];
    assert!(
        write_private_bytes_guarded(
            PrivateFileEntry::ensure(&path).unwrap(),
            &oversized,
            TEST_MAX_BYTES,
            || Ok(()),
        )
        .is_err()
    );
    assert_eq!(std::fs::read(path).unwrap(), FIRST_CONTENT);
}
