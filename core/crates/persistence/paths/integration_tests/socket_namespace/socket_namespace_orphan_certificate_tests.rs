use super::{test_support::RegisteredFixture, *};
use std::fs;

const SAVED_DIRECTORY: &str = "saved-directory";
const SAVED_HOME: &str = "saved-home";
const CERTIFICATE_ATTRIBUTE: &std::ffi::CStr = c"user.yolo.namespace";
const SCHEMA_FIELD: &str = stringify!(schema_version);
const PREVIOUS_CERTIFICATE_SCHEMA: u16 = 2;
const ATTRIBUTE_OPTIONS: libc::c_int = 0;
#[cfg(target_os = "macos")]
const ATTRIBUTE_POSITION: u32 = 0;

fn replace_attribute(directory: &PrivateDirectory, bytes: &[u8]) {
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::fsetxattr(
            directory.descriptor(),
            CERTIFICATE_ATTRIBUTE.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_POSITION,
            ATTRIBUTE_OPTIONS,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::fsetxattr(
            directory.descriptor(),
            CERTIFICATE_ATTRIBUTE.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_OPTIONS,
        )
    };
    assert_eq!(result, libc::c_int::default());
}

#[test]
fn missing_previous_or_copied_certificate_never_authorizes_orphan_cleanup() {
    for variant in [Missing, Previous, Copied] {
        let fixture = RegisteredFixture::new();
        let mut owner = fixture.paths.retain_socket_namespace().unwrap();
        let directory = owner.directory().unwrap();
        let mut scanner = owner.scanner().unwrap();
        let certificate = directory_certificate::read(&directory).unwrap().unwrap();
        owner.preserve_on_release();
        drop(owner);
        let saved = directory.path().parent().unwrap().join(SAVED_DIRECTORY);
        let replacement = if matches!(variant, Previous) {
            directory.try_clone().unwrap()
        } else {
            fs::rename(directory.path(), &saved).unwrap();
            PrivateDirectory::ensure(directory.path()).unwrap()
        };
        match variant {
            Missing => {}
            Previous => {
                let mut value = serde_json::to_value(certificate).unwrap();
                value[SCHEMA_FIELD] = PREVIOUS_CERTIFICATE_SCHEMA.into();
                replace_attribute(&replacement, &serde_json::to_vec(&value).unwrap());
            }
            Copied => replace_attribute(&replacement, &serde_json::to_vec(&certificate).unwrap()),
        }
        finish_scanner_pass(&mut scanner);
        replacement.verify_path().unwrap();
        if !matches!(variant, Previous) {
            assert!(saved.exists());
        }
        if !matches!(variant, Missing) {
            assert!(fixture.paths.retain_socket_namespace().is_err());
            replacement.verify_path().unwrap();
        }
    }
}

#[derive(Clone, Copy)]
enum CertificateCase {
    Missing,
    Previous,
    Copied,
}
use CertificateCase::*;

#[test]
fn same_home_replacement_recovers_empty_transport_only_after_all_participants_leave() {
    let fixture = RegisteredFixture::new();
    let mut owner = fixture.paths.retain_socket_namespace().unwrap();
    let mut peer = fixture.paths.retain_socket_namespace().unwrap();
    let directory = owner.directory().unwrap();
    let certificate = directory_certificate::read(&directory).unwrap().unwrap();
    let home = fixture.paths.home();
    let saved = tempfile::tempdir().unwrap();
    fs::rename(home, saved.path().join(SAVED_HOME)).unwrap();
    PrivateDirectory::ensure(home).unwrap();
    assert!(fixture.paths.retain_socket_namespace().is_err());
    owner.preserve_on_release();
    drop(owner);
    assert!(fixture.paths.retain_socket_namespace().is_err());
    assert_eq!(
        directory_certificate::read(&directory).unwrap(),
        Some(certificate)
    );
    peer.preserve_on_release();
    drop(peer);
    let recovered = fixture.paths.retain_socket_namespace().unwrap();
    recovered.verify().unwrap();
    assert!(!recovered.directory().unwrap().same_identity(&directory));
    drop(saved);
}
