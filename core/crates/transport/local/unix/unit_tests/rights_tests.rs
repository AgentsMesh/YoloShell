use super::{LocalUnixStream, rights};
use std::fs::File;
use std::io::Read;
use std::os::fd::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use tokio::io::AsyncReadExt;

const TEST_MARKER: u8 = b'R';
const WRONG_MARKER: u8 = b'W';
const PIPE_ENDPOINTS: usize = 2;
const PIPE_WRITER: usize = 1;
const SINGLE_DESCRIPTOR_COUNT: usize = 1;
const TEST_BYTE: u8 = b'Y';
const READINESS_DELAY: std::time::Duration = std::time::Duration::from_millis(20);
const READINESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
const SINGLE_EMPTY_ATTEMPT: usize = 1;

#[tokio::test]
async fn descriptor_io_clears_stale_readiness_before_waiting_for_another_frame() {
    use tokio::io::AsyncWriteExt;
    let (mut sender, mut receiver) = LocalUnixStream::pair().unwrap();
    sender.write_all(&[TEST_BYTE]).await.unwrap();
    let mut consumed = [u8::default()];
    receiver.read_exact(&mut consumed).await.unwrap();
    receiver.readable().await.unwrap();
    let [_reader, writer] = pipe();
    let attempts = std::cell::Cell::new(usize::default());
    let receive = receiver.async_io(tokio::io::Interest::READABLE, || {
        attempts.set(attempts.get() + SINGLE_EMPTY_ATTEMPT);
        super::ancillary_recv::receive_exact(
            receiver.as_raw_fd(),
            TEST_MARKER,
            SINGLE_DESCRIPTOR_COUNT,
        )
    });
    let send = async {
        tokio::time::sleep(READINESS_DELAY).await;
        assert_eq!(attempts.get(), SINGLE_EMPTY_ATTEMPT);
        rights::send_exact(&sender, TEST_MARKER, &[writer.as_fd()])
            .await
            .unwrap();
    };
    let (received, ()) =
        tokio::time::timeout(READINESS_TIMEOUT, async { tokio::join!(receive, send) })
            .await
            .unwrap();
    assert_eq!(received.unwrap().len(), SINGLE_DESCRIPTOR_COUNT);
}

#[tokio::test]
async fn exact_right_is_received_cloexec_and_usable() {
    let (sender, receiver) = LocalUnixStream::pair().unwrap();
    let [reader, writer] = pipe();
    let mut reader = File::from(reader);
    rights::send_exact(&sender, TEST_MARKER, &[writer.as_fd()])
        .await
        .unwrap();
    let received = rights::receive_exact(&receiver, TEST_MARKER, 1)
        .await
        .unwrap();
    assert_eq!(received.len(), 1);
    let flags = unsafe { libc::fcntl(received[0].as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
    drop(writer);
    assert_eq!(
        unsafe { libc::write(received[0].as_raw_fd(), &TEST_BYTE as *const u8 as _, 1) },
        1
    );
    let mut byte = u8::default();
    assert_eq!(reader.read(std::slice::from_mut(&mut byte)).unwrap(), 1);
    assert_eq!(byte, TEST_BYTE);
}

#[cfg(target_os = "macos")]
#[test]
fn accepted_descriptor_security_holds_spawn_gate_until_cloexec() {
    let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    let pipe = super::test_support::pipe();
    super::test_support::send_right(sender.as_raw_fd(), pipe.writer.as_raw_fd(), TEST_MARKER);
    drop(pipe.writer);
    let pause = super::test_support::install(receiver.as_raw_fd());
    let reader = std::thread::spawn(move || {
        super::ancillary_recv::receive_exact(
            receiver.as_raw_fd(),
            TEST_MARKER,
            SINGLE_DESCRIPTOR_COUNT,
        )
        .unwrap()
    });

    pause.wait_until_reached();
    let spawn_gate_was_held = super::test_support::spawn_gate_is_held();
    pause.release();
    let received = reader.join().unwrap();

    assert!(spawn_gate_was_held);
    assert_eq!(received.len(), SINGLE_DESCRIPTOR_COUNT);
    let flags = unsafe { libc::fcntl(received[0].as_raw_fd(), libc::F_GETFD) };
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
    drop(received);
    super::test_support::assert_reader_eof(&pipe.reader);
}

#[tokio::test]
async fn wrong_marker_rejects_and_closes_received_right() {
    assert_rejected_and_writer_closed(WRONG_MARKER, 1, 1).await;
}

#[tokio::test]
async fn extra_right_is_rejected_and_every_received_right_closes() {
    assert_rejected_and_writer_closed(TEST_MARKER, PIPE_ENDPOINTS, 1).await;
}

#[tokio::test]
async fn ordinary_transport_rejects_ancillary_data() {
    let (sender, mut receiver) = LocalUnixStream::pair().unwrap();
    let [_reader, writer] = pipe();
    rights::send_exact(&sender, TEST_MARKER, &[writer.as_fd()])
        .await
        .unwrap();
    let mut marker = u8::default();
    let error = receiver
        .read_exact(std::slice::from_mut(&mut marker))
        .await
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    let called = std::cell::Cell::new(false);
    let error = receiver
        .async_io(tokio::io::Interest::READABLE, || {
            called.set(true);
            Ok(())
        })
        .await
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert!(!called.get());
}

async fn assert_rejected_and_writer_closed(marker: u8, sent: usize, expected: usize) {
    let (sender, receiver) = LocalUnixStream::pair().unwrap();
    let pipes = (0..sent).map(|_| pipe()).collect::<Vec<_>>();
    let writers = pipes
        .iter()
        .map(|pair| pair[PIPE_WRITER].as_fd())
        .collect::<Vec<_>>();
    rights::send_exact(&sender, marker, &writers).await.unwrap();
    assert!(
        rights::receive_exact(&receiver, TEST_MARKER, expected)
            .await
            .is_err()
    );
    drop(writers);
    let (readers, writers): (Vec<_>, Vec<_>) = pipes
        .into_iter()
        .map(|[reader, writer]| (reader, writer))
        .unzip();
    drop(writers);
    for reader in readers {
        let mut reader = File::from(reader);
        let mut byte = u8::default();
        assert_eq!(reader.read(std::slice::from_mut(&mut byte)).unwrap(), 0);
    }
}

fn pipe() -> [OwnedFd; PIPE_ENDPOINTS] {
    let mut descriptors = [libc::c_int::default(); PIPE_ENDPOINTS];
    assert_eq!(
        unsafe { libc::pipe(descriptors.as_mut_ptr()) },
        libc::c_int::default()
    );
    descriptors.map(|descriptor| unsafe { OwnedFd::from_raw_fd(descriptor) })
}
