#![cfg(target_os = "macos")]

use std::os::fd::AsRawFd;

const TEST_MARKER: u8 = 0x59;
const TEST_DESCRIPTOR_PATH: &str = "/dev/null";

#[test]
fn receive_pause_hook_is_active_for_external_test_consumers() {
    let (sender, receiver) = std::os::unix::net::UnixStream::pair().unwrap();
    let descriptor = std::fs::File::open(TEST_DESCRIPTOR_PATH).unwrap();
    yolo_local_unix_test_support::test_support::send_right(
        sender.as_raw_fd(),
        descriptor.as_raw_fd(),
        TEST_MARKER,
    );
    let pause = yolo_local_unix_test_support::test_support::install(receiver.as_raw_fd());
    let receive = std::thread::spawn(move || {
        yolo_local_unix_test_support::macos::receive_exactly_one_right(
            receiver.as_raw_fd(),
            TEST_MARKER,
        )
    });
    pause.wait_until_reached();
    pause.release();
    receive.join().unwrap().unwrap();
}
