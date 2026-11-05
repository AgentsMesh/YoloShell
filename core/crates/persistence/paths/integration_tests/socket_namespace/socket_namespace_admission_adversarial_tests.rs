use super::*;
use std::cell::RefCell;

const SAVED_DIRECTORY: &str = "saved-directory";
const SAVED_REGISTRATION: &str = "saved-registration";
const CERTIFICATE_ATTRIBUTE: &std::ffi::CStr = c"user.yolo.namespace";
#[cfg(target_os = "macos")]
const REMOVE_ATTRIBUTE_OPTIONS: libc::c_int = 0;

thread_local! {
    static BEFORE_REGISTRATION: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

pub(crate) fn after_certificate() {
    let mutation = BEFORE_REGISTRATION.with(|slot| slot.borrow_mut().take());
    if let Some(mutation) = mutation {
        mutation();
    }
}

fn mutate_after_certificate(mutation: impl FnOnce() + 'static) {
    BEFORE_REGISTRATION.with(|slot| *slot.borrow_mut() = Some(Box::new(mutation)));
}

fn remove_certificate(directory: &PrivateDirectory) {
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::fremovexattr(
            directory.descriptor(),
            CERTIFICATE_ATTRIBUTE.as_ptr(),
            REMOVE_ATTRIBUTE_OPTIONS,
        )
    };
    #[cfg(target_os = "linux")]
    let result =
        unsafe { libc::fremovexattr(directory.descriptor(), CERTIFICATE_ATTRIBUTE.as_ptr()) };
    assert_eq!(result, libc::c_int::default());
}

#[test]
fn lookup_rejects_directory_replacement_after_initial_certificate_read() {
    let (_home, root, paths) = fixture();
    let mut owner = paths.retain_socket_namespace().unwrap();
    let current = owner.directory.path().to_path_buf();
    let replacement = current.clone();
    let saved = root.path().join(SAVED_DIRECTORY);
    mutate_after_certificate(move || {
        fs::rename(current, saved).unwrap();
        PrivateDirectory::ensure(&replacement).unwrap();
        fs::write(replacement.join(UNKNOWN_FILE), PRESERVED_BYTES).unwrap();
    });
    assert!(paths.host_socket().is_err());
    let replacement = owner.directory.path().join(UNKNOWN_FILE);
    assert_eq!(fs::read(replacement).unwrap(), PRESERVED_BYTES);
    owner.preserve_on_release();
}

#[test]
fn lookup_rejects_certificate_loss_after_initial_certificate_read() {
    let (_home, _root, paths) = fixture();
    let mut owner = paths.retain_socket_namespace().unwrap();
    let directory = owner.directory.try_clone().unwrap();
    mutate_after_certificate(move || remove_certificate(&directory));
    assert!(paths.hub_socket().is_err());
    assert!(owner.verify().is_err());
    owner.preserve_on_release();
}

#[test]
fn lookup_rejects_replacement_registration_even_with_identical_json() {
    let (home, _root, paths) = fixture();
    let mut owner = paths.retain_socket_namespace().unwrap();
    let registration = home.path().join(REGISTRATION_FILE);
    let saved = home.path().join(SAVED_REGISTRATION);
    let bytes = fs::read(&registration).unwrap();
    mutate_after_certificate(move || {
        fs::rename(&registration, saved).unwrap();
        fs::write(&registration, bytes).unwrap();
        fs::set_permissions(registration, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    });
    assert!(paths.host_socket().is_err());
    assert!(owner.verify().is_err());
    owner.preserve_on_release();
}

#[test]
fn live_durable_owner_prevents_recovery_when_certificate_is_lost() {
    let (_home, _root, paths) = fixture();
    let mut owner = paths.retain_socket_namespace().unwrap();
    let inode = fs::metadata(owner.directory.path()).unwrap().ino();
    remove_certificate(&owner.directory);
    assert!(paths.retain_socket_namespace().is_err());
    assert_eq!(fs::metadata(owner.directory.path()).unwrap().ino(), inode);
    owner.preserve_on_release();
}
