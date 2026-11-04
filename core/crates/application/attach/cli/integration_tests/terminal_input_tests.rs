use super::*;
use std::io::Write;
use std::time::Duration;
use tokio::io::AsyncReadExt;

const PAYLOAD: &[u8] = b"terminal-input";
const TIMEOUT: Duration = Duration::from_secs(2);
const CANCEL_DELAY: Duration = Duration::from_millis(10);

#[test]
fn idle_input_does_not_keep_the_runtime_alive_after_cancellation() {
    let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    let (finished, observed) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut input = TerminalInput::new(receiver.as_raw_fd()).unwrap();
            let mut bytes = [0_u8; 1];
            assert!(
                tokio::time::timeout(CANCEL_DELAY, input.read(&mut bytes))
                    .await
                    .is_err()
            );
        });
        drop(runtime);
        finished.send(()).unwrap();
    });
    observed.recv_timeout(TIMEOUT).unwrap();
    worker.join().unwrap();
    drop(sender);
}

#[tokio::test]
async fn input_reads_data_eof_and_restores_the_inherited_flags() {
    let (mut sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    let flags = unsafe { libc::fcntl(receiver.as_raw_fd(), libc::F_GETFL) };
    let mut input = TerminalInput::new(receiver.as_raw_fd()).unwrap();
    sender.write_all(PAYLOAD).unwrap();
    sender.shutdown(std::net::Shutdown::Write).unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(TIMEOUT, input.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bytes, PAYLOAD);
    drop(input);
    assert_eq!(
        unsafe { libc::fcntl(receiver.as_raw_fd(), libc::F_GETFL) },
        flags
    );
}

#[tokio::test]
async fn invalid_input_descriptor_is_rejected() {
    assert!(TerminalInput::new(FAILED).is_err());
}
