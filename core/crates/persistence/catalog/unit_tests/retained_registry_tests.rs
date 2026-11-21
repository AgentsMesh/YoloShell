use super::retained_test_fixture::RetainedFixture;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file::PrivateFileEntry;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::sync::{Arc, Barrier};

const ALIAS_HOME_NAME: &str = "HOME-ALIAS";
const REGISTERING_THREADS: usize = 2;

#[test]
fn registry_isolates_a_textual_alias_for_the_same_parent_descriptor() {
    let fixture = RetainedFixture::new();
    let first =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    let alias = aliased_entry(&fixture);

    let second = super::retained_namespace::RetainedCatalogNamespace::register(alias).unwrap();
    assert!(first.ptr_eq(&second));
    drop(first);
    assert!(super::retained_namespace::is_registered(&fixture.catalog));
    drop(second);
}

#[test]
fn concurrent_equivalent_registrations_share_one_capability() {
    let fixture = RetainedFixture::new();
    let entries = [fixture.entry(), aliased_entry(&fixture)];
    let start = Arc::new(Barrier::new(REGISTERING_THREADS));
    let finish = Arc::new(Barrier::new(REGISTERING_THREADS));
    let handles = entries.map(|entry| {
        let start = Arc::clone(&start);
        let finish = Arc::clone(&finish);
        std::thread::spawn(move || {
            start.wait();
            let result =
                super::retained_namespace::RetainedCatalogNamespace::register(entry).unwrap();
            finish.wait();
            result
        })
    });

    let [first, second] = handles.map(|handle| handle.join().unwrap());
    assert!(first.ptr_eq(&second));
    let registered_path = first.path().to_path_buf();
    drop(first);
    assert!(super::retained_namespace::is_registered(&registered_path));
    drop(second);
    assert!(!super::retained_namespace::is_registered(&registered_path));
}

#[test]
fn different_physical_slot_never_overwrites_a_textual_mapping() {
    let fixture = RetainedFixture::new();
    let original_entry = fixture.entry();
    let original_parent = descriptor_identity(original_entry.parent_descriptor());
    let retained =
        super::retained_namespace::RetainedCatalogNamespace::register(original_entry).unwrap();
    fixture.replace_home();
    let replacement_parent = PrivateDirectory::open(&fixture.home).unwrap();
    let replacement =
        PrivateFileEntry::from_directory(replacement_parent, &fixture.catalog).unwrap();

    assert!(super::retained_namespace::RetainedCatalogNamespace::register(replacement).is_err());
    let resolved = super::retained_namespace::resolve(&fixture.catalog).unwrap();
    assert_eq!(
        descriptor_identity(resolved.entry().parent_descriptor()),
        original_parent
    );
    drop(retained);

    let replacement_parent = PrivateDirectory::open(&fixture.home).unwrap();
    let replacement =
        PrivateFileEntry::from_directory(replacement_parent, &fixture.catalog).unwrap();
    let replacement_parent = descriptor_identity(replacement.parent_descriptor());
    let replacement =
        super::retained_namespace::RetainedCatalogNamespace::register(replacement).unwrap();
    let resolved = super::retained_namespace::resolve(&fixture.catalog).unwrap();
    assert_eq!(
        descriptor_identity(resolved.entry().parent_descriptor()),
        replacement_parent
    );
    drop(replacement);
}

#[test]
fn public_open_rejects_a_retained_sidecar_as_an_independent_catalog() {
    let fixture = RetainedFixture::new();
    let retained =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    let sidecar = fixture.sidecar(super::constants::SQLITE_WAL_SUFFIX);

    assert!(super::Catalog::open(&sidecar).is_err());
    assert!(super::retained_namespace::resolve(&sidecar).is_some());
    drop(retained);
}

#[test]
fn public_open_rejects_a_candidate_sidecar_overlapping_a_retained_main() {
    let fixture = RetainedFixture::new();
    let retained_main = fixture
        .entry()
        .sibling_with_suffix(super::constants::SQLITE_WAL_SUFFIX)
        .unwrap();
    let retained =
        super::retained_namespace::RetainedCatalogNamespace::register(retained_main).unwrap();

    assert!(super::Catalog::open(&fixture.catalog).is_err());
    assert_eq!(
        retained.path(),
        fixture.sidecar(super::constants::SQLITE_WAL_SUFFIX)
    );
    drop(retained);
}

#[test]
fn acquire_error_releases_its_last_owner_after_unlocking_registry() {
    let fixture = RetainedFixture::new();
    let retained =
        super::retained_namespace::RetainedCatalogNamespace::register(fixture.entry()).unwrap();
    fixture.replace_home();
    let replacement_parent = PrivateDirectory::open(&fixture.home).unwrap();
    let replacement =
        PrivateFileEntry::from_directory(replacement_parent, &fixture.catalog).unwrap();
    let acquired = Arc::new(Barrier::new(REGISTERING_THREADS));
    let resume = Arc::new(Barrier::new(REGISTERING_THREADS));
    let thread_acquired = Arc::clone(&acquired);
    let thread_resume = Arc::clone(&resume);
    let handle = std::thread::spawn(move || {
        super::retained_registry::set_acquire_locked(move || {
            thread_acquired.wait();
            thread_resume.wait();
        });
        super::retained_namespace::RetainedCatalogNamespace::acquire(&replacement).is_err()
    });

    acquired.wait();
    drop(retained);
    resume.wait();
    assert!(handle.join().unwrap());
    assert!(!super::retained_namespace::is_registered(&fixture.catalog));
}

fn aliased_entry(fixture: &RetainedFixture) -> PrivateFileEntry {
    let original = fixture.entry();
    let descriptor = unsafe { libc::dup(original.parent_descriptor()) };
    assert!(descriptor >= 0);
    let file = unsafe { File::from_raw_fd(descriptor) };
    let alias_home = fixture.home.with_file_name(ALIAS_HOME_NAME);
    let directory = PrivateDirectory::from_opened(file, &alias_home).unwrap();
    let alias_catalog = alias_home.join(fixture.catalog.file_name().unwrap());
    PrivateFileEntry::from_directory(directory, &alias_catalog).unwrap()
}

fn descriptor_identity(descriptor: std::os::fd::RawFd) -> (libc::dev_t, libc::ino_t) {
    use std::mem::MaybeUninit;

    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    assert_eq!(unsafe { libc::fstat(descriptor, metadata.as_mut_ptr()) }, 0);
    let metadata = unsafe { metadata.assume_init() };
    (metadata.st_dev, metadata.st_ino)
}
