use super::*;

const FCNTL_FAILURE: libc::c_int = -1;
const CLOSE_SUCCESS: libc::c_int = 0;
const RECEIVE_TIMEOUT_DETAIL: &str = "transferred writer was not received";

pub(super) fn run() {
    match std::env::var(HELPER_ROLE_ENV).as_deref() {
        Ok(RECEIVER_ROLE) => receive_writer(),
        Ok(SENDER_ROLE) => send_writer(),
        _ => {}
    }
}

fn receive_writer() {
    std::fs::write(required_path(HELPER_READY_ENV), EMPTY_MARKER).unwrap();
    let writer = receive_writer_before_timeout();
    let flags = unsafe { libc::fcntl(writer.descriptor(), libc::F_GETFD) };
    assert_ne!(flags, FCNTL_FAILURE);
    assert_ne!(flags & libc::FD_CLOEXEC, libc::c_int::default());
    std::fs::write(required_path(HELPER_COMPLETE_ENV), EMPTY_MARKER).unwrap();
    wait_for_path(&required_path(HELPER_TRIGGER_ENV));
    drop(writer);
}

fn receive_writer_before_timeout() -> crate::process::MacosProcessCapabilityChildWriter {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        match crate::descriptor_transfer_test_support::receive_writer_once(libc::STDIN_FILENO) {
            Ok(writer) => return writer,
            Err(error) if is_would_block(&error) => {
                assert!(Instant::now() < deadline, "{RECEIVE_TIMEOUT_DETAIL}");
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(error) => panic!("{error:#}"),
        }
    }
}

fn is_would_block(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|error| error.kind() == std::io::ErrorKind::WouldBlock)
}

fn send_writer() {
    std::fs::write(required_path(HELPER_READY_ENV), EMPTY_MARKER).unwrap();
    wait_for_path(&required_path(HELPER_TRIGGER_ENV));
    let descriptor = std::env::var(HELPER_WRITER_ENV)
        .unwrap()
        .parse::<libc::c_int>()
        .unwrap();
    crate::descriptor_transfer_test_support::send_writer_once(libc::STDIN_FILENO, descriptor).unwrap();
    assert_eq!(unsafe { libc::close(descriptor) }, CLOSE_SUCCESS);
    std::fs::write(required_path(HELPER_COMPLETE_ENV), EMPTY_MARKER).unwrap();
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name).map(PathBuf::from).unwrap()
}
