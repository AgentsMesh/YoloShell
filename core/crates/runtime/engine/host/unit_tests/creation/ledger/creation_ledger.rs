use super::*;
use crate::host::tests::*;

const OVERSIZED_LEDGER_BINDING_COUNT: usize = 4;
const LARGE_LEDGER_NAME_BYTES: usize = crate::protocol::MAX_FRAME_BYTES / 2;

#[test]
fn creation_ledger_is_private_bounded_and_roundtrips() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let mut ledger = CreationLedger::default();
    ledger.bindings.push(ledger_test_binding(directory.path()));
    write_creation_ledger(&path, &ledger).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(CreationLedger::open(&path).unwrap(), ledger);

    std::fs::remove_file(&path).unwrap();
    let oversized = std::fs::File::create(&path).unwrap();
    set_private_file_mode(&path);
    oversized.set_len(MAX_CREATION_LEDGER_BYTES + 1).unwrap();
    assert!(CreationLedger::open(&path).is_err());
}

#[test]
fn deterministic_ledger_capacity_rejection_does_not_poison_the_host() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let large_name = "x".repeat(LARGE_LEDGER_NAME_BYTES);
    let mut oversized = CreationLedger::default();
    for index in 0..OVERSIZED_LEDGER_BINDING_COUNT {
        let mut binding = ledger_test_binding(directory.path());
        binding.creation_token_digest = yolo_crypto::sha256_hex(&index.to_be_bytes());
        binding.spec.name = Some(large_name.clone());
        oversized.bindings.push(binding);
    }
    let mut ledger = CreationLedger::default();
    let error = ledger.install(&path, oversized).unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(!ledger.poisoned);

    let mut small = ledger.clone();
    small.bindings.push(ledger_test_binding(directory.path()));
    ledger.install(&path, small.clone()).unwrap();
    assert_eq!(CreationLedger::open(&path).unwrap(), small);
}

#[test]
fn deterministic_ledger_encoding_rejection_does_not_poison_the_host() {
    use std::os::unix::ffi::OsStringExt;
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let mut invalid = CreationLedger::default();
    let mut binding = ledger_test_binding(directory.path());
    binding.spec.cwd = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', 0xff]));
    invalid.bindings.push(binding);
    let mut ledger = CreationLedger::default();
    let error = ledger.install(&path, invalid).unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INVALID_REQUEST);
    assert!(!ledger.poisoned);

    let small = CreationLedger::default();
    ledger.install(&path, small.clone()).unwrap();
    assert_eq!(CreationLedger::open(&path).unwrap(), small);
}

#[test]
fn creation_ledger_rejects_symlinks_hardlinks_and_unsafe_modes() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let outside = directory.path().join("outside.json");
    std::fs::write(&outside, b"outside").unwrap();
    set_private_file_mode(&outside);
    symlink(&outside, &path).unwrap();
    assert!(CreationLedger::open(&path).is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"outside");

    std::fs::remove_file(&path).unwrap();
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    let other = directory.path().join("ledger-link.json");
    std::fs::hard_link(&path, &other).unwrap();
    assert!(CreationLedger::open(&path).is_err());
    std::fs::remove_file(&other).unwrap();

    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(CreationLedger::open(&path).is_err());
    assert!(write_creation_ledger(&path, &CreationLedger::default()).is_err());
}

#[test]
fn creation_file_identity_rechecks_permissions_after_read() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    write_creation_ledger(&path, &CreationLedger::default()).unwrap();
    let file = std::fs::File::open(&path).unwrap();
    let identity = PrivateFileIdentity::inspect(
        &file.metadata().unwrap(),
        &path,
        "creation ledger",
        MAX_CREATION_LEDGER_BYTES,
    )
    .unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        identity
            .verify(&file.metadata().unwrap(), &path, "creation ledger")
            .is_err()
    );
}

#[test]
fn creation_ledger_rejects_a_symlinked_parent() {
    use std::os::unix::fs::symlink;
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let actual = directory.path().join("actual");
    std::fs::create_dir(&actual).unwrap();
    let alias = directory.path().join("alias");
    symlink(&actual, &alias).unwrap();
    let path = alias.join(CREATION_LEDGER_FILE_NAME);
    assert!(write_creation_ledger(&path, &CreationLedger::default()).is_err());
    assert!(!actual.join(CREATION_LEDGER_FILE_NAME).exists());
}

#[test]
fn creation_ledger_removes_strictly_named_private_stale_temps() {
    let directory = tempdir().unwrap();
    Paths::new(directory.path()).ensure_base().unwrap();
    let ledger_path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    let stale = directory.path().join(format!(
        ".{CREATION_LEDGER_FILE_NAME}.{}.tmp",
        Uuid::new_v4()
    ));
    std::fs::write(&stale, b"stale").unwrap();
    set_private_file_mode(&stale);
    assert_eq!(
        CreationLedger::open(&ledger_path).unwrap(),
        CreationLedger::default()
    );
    assert!(!stale.exists());
}

#[test]
fn invalid_creation_ledger_does_not_advance_controller_generation() {
    let directory = tempdir().unwrap();
    let first = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_eq!(first.controller_generation(), 1);
    drop(first);
    let ledger_path = directory.path().join(CREATION_LEDGER_FILE_NAME);
    std::fs::write(&ledger_path, b"{").unwrap();
    set_private_file_mode(&ledger_path);
    assert!(Host::open(HostConfig::new(directory.path())).is_err());
    assert!(Host::open(HostConfig::new(directory.path())).is_err());
    let catalog = Catalog::open(&Paths::new(directory.path()).catalog()).unwrap();
    assert_eq!(catalog.controller_generation().unwrap(), 1);
    drop(catalog);
    std::fs::remove_file(&ledger_path).unwrap();
    let second = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_eq!(second.controller_generation(), 2);
}
