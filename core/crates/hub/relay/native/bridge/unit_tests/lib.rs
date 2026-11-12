use crate::*;
mod request_roundtrip;
mod startup;

#[test]
fn null_ffi_inputs_are_rejected_without_mutating_external_state() {
    assert_eq!(unsafe { yolo_native_relay_encode_request(std::ptr::null(), std::ptr::null_mut()) }, INVALID);
    assert_eq!(unsafe { yolo_native_relay_decode(std::ptr::null(), usize::MAX, std::ptr::null_mut()) }, INVALID);
    unsafe { yolo_native_relay_release_buffer(std::ptr::null_mut()); yolo_native_relay_release_decoded(std::ptr::null_mut()); }
}

#[test]
fn typed_status_roundtrips_without_json_and_releases_all_owned_memory() {
    let mut output = types::Buffer::default();
    let holder = b"owned-holder";
    assert_eq!(unsafe { yolo_native_relay_encode_status(protocol::StatusKind::ReadOnly as u32,
        types::Text { data: holder.as_ptr(), length: holder.len() }, &mut output) }, OK);
    let mut decoded = types::Decoded::default();
    assert_eq!(unsafe { yolo_native_relay_decode(output.data, output.length, &mut decoded) }, OK);
    assert_eq!(decoded.kind, STATUS);
    assert_eq!(unsafe { std::slice::from_raw_parts(decoded.holder.data, decoded.holder.length) }, holder);
    unsafe { yolo_native_relay_release_decoded(&mut decoded); yolo_native_relay_release_buffer(&mut output); }
    assert!(output.data.is_null()); assert!(decoded.holder.data.is_null());
}
