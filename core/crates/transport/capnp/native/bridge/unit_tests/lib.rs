use super::*;
use std::ptr;
mod native_access;
mod native_ownership;

#[test]
fn owned_structural_tree_roundtrips_without_json_or_loss_of_integer_precision() {
    unsafe {
        let root = yolo_native_value_map();
        let key = yolo_native_value_text(b"revision".as_ptr(), b"revision".len());
        assert_eq!(
            yolo_native_value_insert(root, key, yolo_native_value_unsigned(u64::MAX)),
            OK
        );
        let mut buffer = Buffer::default();
        assert_eq!(yolo_native_value_encode(root, &mut buffer), OK);
        let mut decoded = ptr::null_mut();
        assert_eq!(
            yolo_native_value_decode(buffer.data, buffer.length, &mut decoded),
            OK
        );
        assert_eq!(yolo_native_value_tag(decoded), MAP);
        assert_eq!(yolo_native_value_get_count(decoded), 1);
        assert_eq!(
            yolo_native_value_get_u64(yolo_native_value_get_value(decoded, 0)),
            u64::MAX
        );
        assert!(yolo_native_value_get_key(decoded, 1).is_null());
        yolo_native_value_release(decoded);
        yolo_native_value_release(root);
        yolo_native_value_release_buffer(&mut buffer);
        assert!(buffer.data.is_null());
        yolo_native_value_release_buffer(&mut buffer);
    }
}

#[test]
fn invalid_input_and_failed_ownership_transfers_leave_the_original_roots_owned() {
    unsafe {
        let root = yolo_native_value_null();
        let child = yolo_native_value_boolean(true);
        assert_eq!(yolo_native_value_push(root, child), INVALID);
        assert!(yolo_native_value_get_bool(child));
        assert_eq!(yolo_native_value_push(child, child), INVALID);
        let mut decoded = root;
        assert_eq!(
            yolo_native_value_decode(ptr::null(), 1, &mut decoded),
            INVALID
        );
        assert!(decoded.is_null());
        assert!(yolo_native_value_text(b"\xff".as_ptr(), 1).is_null());
        assert!(
            yolo_native_value_data(ptr::null(), yolo_capnp_codec::MAX_MESSAGE_BYTES + 1).is_null()
        );
        yolo_native_value_release(child);
        yolo_native_value_release(root);
        yolo_native_value_release(ptr::null_mut());
    }
}

#[test]
fn duplicate_keys_in_untrusted_structural_documents_are_rejected_before_native_mapping() {
    let duplicate = Value::Map(vec![
        (Value::from("id"), Value::from(1u64)),
        (Value::from("id"), Value::from(2u64)),
    ]);
    let bytes = yolo_capnp_codec::encode_value(&duplicate).unwrap();
    let mut decoded = ptr::null_mut();
    unsafe {
        assert_eq!(
            yolo_native_value_decode(bytes.as_ptr(), bytes.len(), &mut decoded),
            INVALID
        );
        assert!(decoded.is_null());
    }
}

#[test]
fn native_byte_slices_need_not_have_word_alignment() {
    let bytes = yolo_capnp_codec::encode(&42u64).unwrap();
    let mut unaligned = vec![0];
    unaligned.extend(bytes);
    let mut root = ptr::null_mut();
    unsafe {
        assert_eq!(
            yolo_native_value_decode(unaligned[1..].as_ptr(), unaligned.len() - 1, &mut root),
            OK
        );
        assert_eq!(yolo_native_value_get_u64(root), 42);
        yolo_native_value_release(root);
    }
}
