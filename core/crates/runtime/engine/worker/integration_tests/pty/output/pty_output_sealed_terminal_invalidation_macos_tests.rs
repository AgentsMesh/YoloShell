use super::sealed_terminal_invalidation_support::*;
use super::*;
use std::io::Write;

const LATE_SEALED_OUTPUT: &[u8] = b"late-sealed-output";
const SEALED_OPERATION_REACHED_DETAIL: &str =
    "sealed EOF operation ran after terminal continuity was disproved";

#[test]
fn first_seal_terminal_false_revokes_reader_identity() {
    let (_directory, mut output, lifecycle, version, path) = reader_fixture(true);
    let stale = lifecycle.kernel_eof_identity().unwrap();
    let mut authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let mut reopened = None;

    assert!(
        !output
            .commit_if_kernel_eof_current(&mut authority, &version, |validate| {
                reject_after_reopen(validate, &path, &mut reopened)
            })
            .unwrap()
    );

    assert!(lifecycle.kernel_eof_observation().is_none());
    assert!(
        !output
            .validates_kernel_eof_authority(&authority, &version)
            .unwrap()
    );
    drop(reopened);
    assert_fresh_reader_identity(&mut output, &version, &stale);
}

#[test]
fn first_seal_terminal_false_revokes_probe_identity() {
    let (_directory, mut output, _lifecycle, version, path) = reader_fixture(false);
    let mut authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let stale = authority.identity_for_test();
    let mut reopened = None;

    assert!(
        !output
            .commit_if_kernel_eof_current(&mut authority, &version, |validate| {
                reject_after_reopen(validate, &path, &mut reopened)
            })
            .unwrap()
    );

    drop(reopened);
    assert_fresh_reader_identity(&mut output, &version, &stale);
}

#[test]
fn first_seal_terminal_false_revokes_exclusive_identity() {
    let (_directory, mut output, version, path) = exclusive_fixture();
    let mut authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let stale = authority.identity_for_test();
    let mut reopened = None;

    assert!(
        !output
            .commit_if_kernel_eof_current(&mut authority, &version, |validate| {
                reject_after_reopen(validate, &path, &mut reopened)
            })
            .unwrap()
    );

    assert!(output.kernel_eof_identity_for_test().is_none());
    drop(reopened);
    let fresh = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&stale, &fresh.identity_for_test()));
}

#[test]
fn existing_reader_seal_is_revoked_before_retry_after_silent_reopen() {
    let (_directory, mut output, lifecycle, version, path) = reader_fixture(true);
    let stale = install_reader_seal(&mut output, &version);
    let mut authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let reopened = open_slave(&path);

    assert!(
        !output
            .commit_if_kernel_eof_current(&mut authority, &version, |_| {
                panic!("{SEALED_OPERATION_REACHED_DETAIL}")
            })
            .unwrap()
    );

    assert!(lifecycle.kernel_eof_observation().is_none());
    drop(reopened);
    assert_fresh_reader_identity(&mut output, &version, &stale);
}

#[test]
fn existing_exclusive_seal_is_revoked_before_retry_after_late_bytes() {
    let (_directory, mut output, version, path) = exclusive_fixture();
    let stale = install_exclusive_seal(&mut output, &version);
    let mut authority = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    let mut reopened = open_slave(&path);
    reopened.write_all(LATE_SEALED_OUTPUT).unwrap();

    assert!(
        !output
            .commit_if_kernel_eof_current(&mut authority, &version, |_| {
                panic!("{SEALED_OPERATION_REACHED_DETAIL}")
            })
            .unwrap()
    );
    assert!(output.kernel_eof_identity_for_test().is_none());
    assert!(
        output
            .acquire_kernel_eof_authority(&version)
            .unwrap()
            .is_none()
    );
    drop(reopened);
    let fresh = output
        .acquire_kernel_eof_authority(&version)
        .unwrap()
        .unwrap();
    assert!(!Arc::ptr_eq(&stale, &fresh.identity_for_test()));
}
