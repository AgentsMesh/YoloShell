use super::runtime_record_test_support::record_for_identity;
use super::*;
use std::io::{ErrorKind, Read};
use std::os::unix::net::UnixListener;
use std::time::Instant;

const PEER_TEST_SOCKET_NAME: &str = "peer-mismatch.sock";
const PEER_TEST_EXECUTABLE: &str = "/bin/sleep";
const PEER_TEST_ARGUMENT: &str = "60";
const PEER_TEST_OBSERVATION_TIMEOUT: Duration = Duration::from_millis(200);
const PEER_TEST_POLL_INTERVAL: Duration = Duration::from_millis(5);
const PEER_TEST_BYTE_COUNT: usize = 1;
const PEER_TEST_NONBLOCKING_CONTEXT: &str = "configure peer observation socket";
const PEER_TEST_READ_CONTEXT: &str = "read peer observation socket";

#[test]
fn mismatched_unix_peer_never_receives_cleanup_authentication() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let socket_directory = tempfile::tempdir().unwrap();
    let socket = socket_directory.path().join(PEER_TEST_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let mut worker = ChildGuard::new(
        Command::new(PEER_TEST_EXECUTABLE)
            .arg(PEER_TEST_ARGUMENT)
            .spawn()
            .unwrap(),
    );
    let identity = ProcessIdentity::capture(worker.id()).unwrap();
    let mut record = record_for_identity(&paths, Uuid::new_v4(), identity);
    record.socket_path = socket;
    let server = std::thread::spawn(move || peer_received_data(listener));

    assert!(!request_worker_termination(&record));
    assert!(!server.join().unwrap());
    assert!(identity.matches());
    worker.kill().unwrap();
    worker.wait().unwrap();
}

fn peer_received_data(listener: UnixListener) -> bool {
    let (mut stream, _) = listener.accept().unwrap();
    stream
        .set_nonblocking(true)
        .expect(PEER_TEST_NONBLOCKING_CONTEXT);
    let deadline = Instant::now() + PEER_TEST_OBSERVATION_TIMEOUT;
    let mut byte = [0_u8; PEER_TEST_BYTE_COUNT];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => return false,
            Ok(_) => return true,
            Err(error) if error.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                std::thread::sleep(PEER_TEST_POLL_INTERVAL);
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => return false,
            Err(error) => panic!("{PEER_TEST_READ_CONTEXT}: {error}"),
        }
    }
}
