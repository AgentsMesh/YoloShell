use crate::*;

#[test]
fn startup_commit_is_bounded_binary_and_not_its_own_acknowledgement() {
    let mut buffer = types::Buffer::default();
    let process = std::process::id();
    assert_eq!(unsafe { startup::yolo_native_startup_encode_commit(process, &mut buffer) }, OK);
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.length) };
    assert_eq!(bytes, yolo_startup_guard::encode_commit(process).unwrap());
    assert_eq!(unsafe { startup::yolo_native_startup_validate_committed(buffer.data, buffer.length, process) }, INVALID);
    assert!(buffer.length <= startup::yolo_native_startup_maximum_frame_bytes());
    unsafe { yolo_native_relay_release_buffer(&mut buffer); }
    assert_eq!(unsafe { startup::yolo_native_startup_encode_commit(0, &mut buffer) }, INVALID);
    assert_eq!(unsafe { startup::yolo_native_startup_validate_committed(std::ptr::null(), usize::MAX, process) }, INVALID);
}
