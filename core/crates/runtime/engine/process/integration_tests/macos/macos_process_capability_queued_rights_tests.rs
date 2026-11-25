use super::{MacosProcessCapabilityIssuer, MacosProcessCapabilityRelease, test_support};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

const QUEUED_WRITER_VISIBLE_DETAIL: &str =
    "queued SCM_RIGHTS writer remained visible in the process descriptor table";
const QUEUED_WRITER_RELEASED_DETAIL: &str =
    "queued SCM_RIGHTS writer was mistaken for a released capability";
const RECEIVED_WRITER_IDENTITY_DETAIL: &str =
    "received SCM_RIGHTS writer did not preserve the exact capability identity";
const RECEIVED_WRITER_MISSING_DETAIL: &str =
    "received SCM_RIGHTS writer was absent from the process descriptor table";
const MATERIALIZED_WRITER_RELEASED_DETAIL: &str =
    "materialized SCM_RIGHTS writer was mistaken for a released capability";
const FINAL_WRITER_VISIBLE_DETAIL: &str =
    "closed SCM_RIGHTS writer remained visible in the process descriptor table";
const FINAL_RELEASE_MISSING_DETAIL: &str =
    "capability did not release after the received SCM_RIGHTS writer closed";

#[test]
fn queued_scm_rights_writer_remains_live_until_the_received_copy_closes() {
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    let expected = writer.identity();
    let (sender, receiver) = UnixStream::pair().unwrap();
    crate::descriptor_transfer_test_support::send_writer_once(
        sender.as_raw_fd(),
        writer.descriptor(),
    )
    .unwrap();
    drop(writer);
    drop(sender);

    assert!(
        !crate::process::process_holds_capability_for_test(
            std::process::id(),
            expected.writer_handle(),
        )
        .unwrap(),
        "{QUEUED_WRITER_VISIBLE_DETAIL}"
    );
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent,
        "{QUEUED_WRITER_RELEASED_DETAIL}"
    );

    let received =
        crate::descriptor_transfer_test_support::receive_writer_once(receiver.as_raw_fd()).unwrap();
    assert_eq!(
        received.identity(),
        expected,
        "{RECEIVED_WRITER_IDENTITY_DETAIL}"
    );
    assert!(
        crate::process::process_holds_capability_for_test(
            std::process::id(),
            expected.writer_handle(),
        )
        .unwrap(),
        "{RECEIVED_WRITER_MISSING_DETAIL}"
    );
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::WriterPresent,
        "{MATERIALIZED_WRITER_RELEASED_DETAIL}"
    );

    drop(received);
    assert!(
        !crate::process::process_holds_capability_for_test(
            std::process::id(),
            expected.writer_handle(),
        )
        .unwrap(),
        "{FINAL_WRITER_VISIBLE_DETAIL}"
    );
    assert_eq!(
        reader.poll_release().unwrap(),
        MacosProcessCapabilityRelease::Released,
        "{FINAL_RELEASE_MISSING_DETAIL}"
    );
}
