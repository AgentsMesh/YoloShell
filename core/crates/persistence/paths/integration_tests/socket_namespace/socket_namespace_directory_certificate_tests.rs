use super::{fixture::*, *};
use std::fs;

#[test]
fn immutable_directory_certificate_survives_regular_file_cleanup() {
    let root = tempfile::tempdir().unwrap();
    let directory = PrivateDirectory::ensure(&root.path().join(ORIGINAL_DIRECTORY)).unwrap();
    assert!(read(&directory).unwrap().is_none());
    let expected = binding();
    install_or_verify(&directory, &expected, registration(&directory)).unwrap();
    let original = native_xattr::read(directory.descriptor()).unwrap().unwrap();
    fs::write(directory.path().join(MARKER), MARKER_BYTES).unwrap();
    fs::remove_file(directory.path().join(MARKER)).unwrap();
    let observed = read(&directory).unwrap().unwrap();
    assert_eq!(observed.binding, expected);
    assert_eq!(observed.registration, registration(&directory));
    install_or_verify(&directory, &expected, registration(&directory)).unwrap();
    assert_eq!(
        native_xattr::read(directory.descriptor()).unwrap().unwrap(),
        original
    );
    let mut conflicting = expected;
    conflicting.home_digest = "b".repeat(super::super::SOCKET_FULL_HASH_HEX_LENGTH);
    assert!(install_or_verify(&directory, &conflicting, registration(&directory)).is_err());
    assert_eq!(
        native_xattr::read(directory.descriptor()).unwrap().unwrap(),
        original
    );
}

#[test]
fn malformed_oversized_or_copied_certificate_is_preserved_and_never_replaced() {
    for bytes in [INVALID_BYTES.to_vec(), vec![b'x'; OVERSIZE_BYTES]] {
        let root = tempfile::tempdir().unwrap();
        let directory = PrivateDirectory::ensure(&root.path().join(ORIGINAL_DIRECTORY)).unwrap();
        write_attribute(&directory, &bytes);
        assert!(read(&directory).is_err());
        assert!(install_or_verify(&directory, &binding(), registration(&directory)).is_err());
        if bytes.len() <= ATTRIBUTE_MAX_BYTES {
            assert_eq!(
                native_xattr::read(directory.descriptor()).unwrap().unwrap(),
                bytes
            );
        }
    }
    let root = tempfile::tempdir().unwrap();
    let first = PrivateDirectory::ensure(&root.path().join(ORIGINAL_DIRECTORY)).unwrap();
    install_or_verify(&first, &binding(), registration(&first)).unwrap();
    let bytes = native_xattr::read(first.descriptor()).unwrap().unwrap();
    let second = PrivateDirectory::ensure(&root.path().join(OTHER_DIRECTORY)).unwrap();
    write_attribute(&second, &bytes);
    assert!(read(&second).is_err());
    assert!(install_or_verify(&second, &binding(), registration(&second)).is_err());
    assert_eq!(
        native_xattr::read(second.descriptor()).unwrap().unwrap(),
        bytes
    );
}

#[test]
fn renamed_directory_and_replacement_cannot_be_adopted_through_an_old_handle() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(ORIGINAL_DIRECTORY);
    let original = PrivateDirectory::ensure(&path).unwrap();
    install_or_verify(&original, &binding(), registration(&original)).unwrap();
    fs::rename(&path, root.path().join(SAVED_DIRECTORY)).unwrap();
    let replacement = PrivateDirectory::ensure(&path).unwrap();
    assert!(read(&original).is_err());
    assert!(install_or_verify(&original, &binding(), registration(&original)).is_err());
    assert!(read(&replacement).unwrap().is_none());
}
