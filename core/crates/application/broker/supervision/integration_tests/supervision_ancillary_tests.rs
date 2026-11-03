use super::ancillary_support::{ANCILLARY_MARKER, Pipe, assert_reader_eof, pipe, send_right};
use super::*;
use std::io::{ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, IntoRawFd};
use std::time::{Duration, Instant};
use yolo_local_unix::BlockingLocalUnixStream;

const TEST_TIMEOUT: Duration = Duration::from_secs(1);
const FCNTL_FAILURE: libc::c_int = -1;

fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}

#[test]
fn child_activation_rejects_right_closes_it_and_poisons_clone() {
    let (parent, child) = std::os::unix::net::UnixStream::pair().unwrap();
    let receiver = ActivationReceiver::inherit(child.into_raw_fd()).unwrap();
    assert_close_on_exec(&receiver.0);
    let mut clone = receiver.0.try_clone().unwrap();
    let transferred = pipe();
    send_right(parent.as_raw_fd(), transferred.writer.as_raw_fd());
    drop(transferred.writer);

    let error = receiver.await_activation().err().unwrap();

    assert_anyhow_io_kind(&error, ErrorKind::InvalidData);
    assert_reader_eof(&transferred.reader);
    assert_eq!(
        clone.write_all(&[ANCILLARY_MARKER]).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
}

#[test]
fn parent_readiness_rejects_right_closes_it_and_poisons_clone() {
    let (parent, publisher) = BlockingLocalUnixStream::pair().unwrap();
    let mut control = ActivationSender(parent);
    let mut clone = control.0.try_clone().unwrap();
    let transferred = pipe();
    send_right(publisher.as_raw_fd(), transferred.writer.as_raw_fd());
    drop(transferred.writer);

    let error = control.read_readiness(deadline()).unwrap_err();

    assert_anyhow_io_kind(&error, ErrorKind::InvalidData);
    assert_reader_eof(&transferred.reader);
    assert_eq!(
        clone.write_all(&[ANCILLARY_MARKER]).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
}

#[test]
fn adoption_prepare_rejects_right_closes_it_and_poisons_clone() {
    let (parent, mut broker) = BlockingLocalUnixStream::pair().unwrap();
    let mut control = ActivationSender(parent);
    let mut clone = control.0.try_clone().unwrap();
    let Pipe { reader, writer } = pipe();
    let injector = std::thread::spawn(move || {
        let request: Control = frame::read(&mut broker).unwrap();
        assert!(matches!(request, Control::Adopt { .. }));
        send_right(broker.as_raw_fd(), writer.as_raw_fd());
        drop(writer);
    });

    let error = control
        .prepare_adoption(uuid::Uuid::new_v4(), deadline())
        .err()
        .unwrap();
    injector.join().unwrap();

    assert_anyhow_io_kind(&error, ErrorKind::InvalidData);
    assert_reader_eof(&reader);
    assert_eq!(
        clone.write_all(&[ANCILLARY_MARKER]).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
}

fn assert_anyhow_io_kind(error: &anyhow::Error, expected: ErrorKind) {
    assert_eq!(
        error.downcast_ref::<std::io::Error>().map(|io| io.kind()),
        Some(expected)
    );
}

fn assert_close_on_exec(stream: &BlockingLocalUnixStream) {
    let flags = unsafe { libc::fcntl(stream.as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
}
