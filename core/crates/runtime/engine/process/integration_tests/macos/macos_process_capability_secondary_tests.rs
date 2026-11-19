use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

const MINIMUM_TEST_DESCRIPTOR: libc::c_int = libc::STDERR_FILENO + 1;
const FCNTL_FAILURE: libc::c_int = -1;
const WRITER_ACCEPTED_DETAIL: &str = "capability writer was accepted as its manager reader";
const OTHER_READER_ACCEPTED_DETAIL: &str = "reader from another capability pair was accepted";

#[test]
fn secondary_reader_duplicate_is_cloexec_and_exact() {
    let (reader, _writer) = staged();
    let expected = reader.identity();

    let descriptor = reader.duplicate_for_secondary().unwrap();
    assert_cloexec(descriptor.as_raw_fd());
    let transferred =
        MacosProcessCapabilityManagerReader::from_transferred_exact(descriptor, expected).unwrap();

    assert_eq!(transferred.identity(), expected);
    assert_cloexec(transferred.descriptor());
}

#[test]
fn writer_role_is_rejected_and_the_descriptor_is_closed() {
    let (reader, writer) = staged();
    let descriptor = duplicate(writer.descriptor());
    let raw = descriptor.as_raw_fd();

    MacosProcessCapabilityManagerReader::from_transferred_exact(descriptor, reader.identity())
        .err()
        .expect(WRITER_ACCEPTED_DETAIL);

    assert_closed(raw);
}

#[test]
fn reader_from_another_pair_is_rejected_and_closed() {
    let (expected, _expected_writer) = staged();
    let (other, _other_writer) = staged();
    let descriptor = other.duplicate_for_secondary().unwrap();
    let raw = descriptor.as_raw_fd();

    MacosProcessCapabilityManagerReader::from_transferred_exact(descriptor, expected.identity())
        .err()
        .expect(OTHER_READER_ACCEPTED_DETAIL);

    assert_closed(raw);
}

#[test]
fn secondary_reader_is_the_only_extra_alias_and_releases_on_drop() {
    let (reader, _writer) = staged();
    let expected = reader.identity();
    let descriptor = reader.duplicate_for_secondary().unwrap();
    let raw = descriptor.as_raw_fd();
    drop(reader);
    assert!(holds_reader(expected));

    let transferred =
        MacosProcessCapabilityManagerReader::from_transferred_exact(descriptor, expected).unwrap();
    assert!(holds_reader(expected));

    drop(transferred);
    assert_closed(raw);
    assert!(!holds_reader(expected));
}

#[test]
fn released_writer_still_allows_an_attested_secondary_duplicate() {
    let (reader, writer) = staged();
    let expected = reader.identity();
    drop(writer);

    let descriptor = reader.duplicate_for_secondary().unwrap();
    let transferred =
        MacosProcessCapabilityManagerReader::from_transferred_attested(descriptor, expected)
            .unwrap();

    assert_eq!(transferred.identity(), expected);
}

fn staged() -> (
    Arc<MacosProcessCapabilityManagerReader>,
    MacosProcessCapabilityChildWriter,
) {
    MacosProcessCapabilityIssuer::create()
        .unwrap()
        .into_staged()
        .unwrap()
}

fn duplicate(source: libc::c_int) -> OwnedFd {
    let raw = unsafe { libc::fcntl(source, libc::F_DUPFD, MINIMUM_TEST_DESCRIPTOR) };
    assert_ne!(raw, FCNTL_FAILURE);
    unsafe { OwnedFd::from_raw_fd(raw) }
}

fn holds_reader(identity: MacosProcessCapabilityIdentity) -> bool {
    crate::process::process_holds_capability_for_test(std::process::id(), identity.reader_handle())
        .unwrap()
}

fn assert_cloexec(descriptor: libc::c_int) {
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}

fn assert_closed(descriptor: libc::c_int) {
    assert_eq!(
        unsafe { libc::fcntl(descriptor, libc::F_GETFD) },
        FCNTL_FAILURE
    );
}
