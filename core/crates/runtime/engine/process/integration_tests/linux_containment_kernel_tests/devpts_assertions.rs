use super::devpts_fixture::STALE_PAYLOAD;
use super::support::{KERNEL_POLL_INTERVAL, KERNEL_SETTLE_TIMEOUT};
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::time::Instant;

const READ_CHUNK_BYTES: usize = 64;

pub fn assert_missing_eof(master: &mut File) {
    let mut byte = [u8::default(); 1];
    let result = master.read(&mut byte);
    assert!(
        matches!(result, Err(ref error) if error.kind() == ErrorKind::WouldBlock),
        "open slave must keep kernel PTY EOF unavailable: {result:?}"
    );
}

pub fn assert_no_bytes(master: &mut File) {
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    let mut byte = [u8::default(); 1];
    while Instant::now() < deadline {
        match master.read(&mut byte) {
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            Ok(0) => {}
            result => panic!("frozen stale writer produced terminal bytes: {result:?}"),
        }
        std::thread::sleep(KERNEL_POLL_INTERVAL);
    }
}

pub fn read_payload(master: &mut File) -> Vec<u8> {
    let deadline = Instant::now() + KERNEL_SETTLE_TIMEOUT;
    let mut bytes = Vec::new();
    while bytes.len() < STALE_PAYLOAD.len() && Instant::now() < deadline {
        let mut chunk = [u8::default(); READ_CHUNK_BYTES];
        match master.read(&mut chunk) {
            Ok(count) => bytes.extend_from_slice(&chunk[..count]),
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(KERNEL_POLL_INTERVAL)
            }
            result => panic!("read reused devpts master failed: {result:?}"),
        }
    }
    bytes
}
