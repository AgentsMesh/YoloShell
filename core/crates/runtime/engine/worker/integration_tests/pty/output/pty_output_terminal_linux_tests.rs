use super::*;
use std::io::{Read, Write};
#[path = "../../../test_support/integration/linux/linux_output_terminal.rs"]
mod fixture;
use fixture::{Fixture, PAYLOAD, TEST_ROUNDS};

#[test]
fn linux_queue_observation_drains_accepted_bytes_before_accepting_eof() {
    for _ in 0..TEST_ROUNDS {
        let Fixture {
            mut master,
            mut slave,
            ..
        } = Fixture::new();
        assert_eq!(
            output_queue_state(&master).unwrap(),
            OutputQueueState::Empty
        );
        assert!(!validates_kernel_pty_eof(&master).unwrap());
        slave.write_all(PAYLOAD).unwrap();
        assert_eq!(
            output_queue_state(&master).unwrap(),
            OutputQueueState::Pending
        );
        drop(slave);
        assert!(!validates_kernel_pty_eof(&master).unwrap());
        let mut received = vec![u8::default(); PAYLOAD.len()];
        master.read_exact(&mut received).unwrap();
        assert_eq!(received, PAYLOAD);
        assert_eq!(
            output_queue_state(&master).unwrap(),
            OutputQueueState::Empty
        );
        let error = master
            .read(&mut [u8::default(); fixture::EMPTY_READ])
            .unwrap_err();
        assert!(is_kernel_pty_eof_error(&error));
        assert!(validates_consumed_kernel_pty_eof(&master).unwrap());
    }
}

#[test]
fn linux_reopened_slave_revokes_eof_before_emitting_more_output() {
    let Fixture {
        mut master,
        slave,
        path,
    } = Fixture::new();
    drop(slave);
    assert!(validates_kernel_pty_eof(&master).unwrap());
    let mut replacement = fixture::open_slave(&path);
    assert!(!validates_consumed_kernel_pty_eof(&master).unwrap());
    replacement.write_all(PAYLOAD).unwrap();
    assert_eq!(
        output_queue_state(&master).unwrap(),
        OutputQueueState::Pending
    );
    let mut received = vec![u8::default(); PAYLOAD.len()];
    master.read_exact(&mut received).unwrap();
    assert_eq!(received, PAYLOAD);
    assert!(!validates_consumed_kernel_pty_eof(&master).unwrap());
    drop(replacement);
    assert!(validates_consumed_kernel_pty_eof(&master).unwrap());
}

#[test]
fn linux_queue_errors_cannot_be_misclassified_as_an_empty_terminal() {
    let ordinary = tempfile::tempfile().unwrap();
    assert!(output_queue_state(&ordinary).is_err());
    assert!(!validates_kernel_pty_eof(&ordinary).unwrap());
    assert!(!is_kernel_pty_eof_error(&io::Error::from_raw_os_error(
        libc::EBADF
    )));
}
