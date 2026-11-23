use super::*;

const TRANSIENT_REGISTRATION_ATTEMPTS: usize = 8;

#[cfg(target_os = "macos")]
#[test]
fn lease_is_exclusive_while_held() {
    let name = format!("mach-test-{}.lock", uuid::Uuid::new_v4());
    let path = Path::new("/private/tmp").join(name);
    let first = CoordinationLease::try_acquire(&path).unwrap().unwrap();
    assert!(CoordinationLease::try_acquire(&path).unwrap().is_none());
    drop(first);
}

#[cfg(target_os = "macos")]
#[test]
fn same_file_name_in_distinct_parents_has_distinct_lease() {
    let name = format!("mach-test-{}.lock", uuid::Uuid::new_v4());
    let first_path = Path::new("/private/tmp/first").join(&name);
    let second_path = Path::new("/private/tmp/second").join(name);
    let first = CoordinationLease::try_acquire(&first_path)
        .unwrap()
        .unwrap();
    let second = CoordinationLease::try_acquire(&second_path)
        .unwrap()
        .unwrap();
    drop((first, second));
}

#[cfg(target_os = "macos")]
#[test]
fn registration_statuses_are_classified_fail_closed() {
    assert_eq!(
        registration_outcome(BOOTSTRAP_SUCCESS, || Ok(false)).unwrap(),
        RegistrationOutcome::Acquired
    );
    assert_eq!(
        registration_outcome(BOOTSTRAP_NAME_IN_USE, || Ok(false)).unwrap(),
        RegistrationOutcome::Busy
    );
    assert_eq!(
        registration_outcome(BOOTSTRAP_SERVICE_ACTIVE, || Ok(false)).unwrap(),
        RegistrationOutcome::Busy
    );
    assert_eq!(
        registration_outcome(BOOTSTRAP_NOT_PRIVILEGED, || Ok(true)).unwrap(),
        RegistrationOutcome::Busy
    );
    assert_eq!(
        registration_outcome(BOOTSTRAP_NOT_PRIVILEGED, || Ok(false)).unwrap(),
        RegistrationOutcome::Retry
    );
    assert!(registration_outcome(libc::KERN_FAILURE, || Ok(false)).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn transient_registration_ambiguity_is_nonfatal_and_never_acquired() {
    for _ in 0..TRANSIENT_REGISTRATION_ATTEMPTS {
        assert!(!register_service(|| BOOTSTRAP_NOT_PRIVILEGED, || Ok(false)).unwrap());
    }
    assert!(
        !register_service(
            || BOOTSTRAP_NOT_PRIVILEGED,
            || { lookup_status_is_registered(libc::KERN_INVALID_RIGHT) }
        )
        .unwrap()
    );
    assert!(register_service(|| BOOTSTRAP_SUCCESS, || Ok(false)).unwrap());
}

#[cfg(target_os = "macos")]
#[test]
fn lookup_statuses_accept_only_absence_and_retired_rights() {
    assert!(lookup_status_is_registered(BOOTSTRAP_SUCCESS).unwrap());
    assert!(!lookup_status_is_registered(BOOTSTRAP_UNKNOWN_SERVICE).unwrap());
    assert!(!lookup_status_is_registered(libc::KERN_INVALID_RIGHT).unwrap());
    assert!(lookup_status_is_registered(libc::KERN_FAILURE).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn a_unique_service_name_is_observed_as_unregistered() {
    let name = format!("mach-lookup-test-{}", uuid::Uuid::new_v4());
    let service = service_name(Path::new(&name)).unwrap();

    assert!(!service_is_registered(&service).unwrap());
}
