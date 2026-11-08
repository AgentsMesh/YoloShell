use super::*;

const SECRET: &[u8] = b"private-native-value";
const KEY: &[u8] = b"credential";

#[test]
fn failed_map_insertions_keep_every_original_owner_live_and_do_not_replace_the_first_value() {
    unsafe {
        let root = yolo_native_value_map();
        let key = yolo_native_value_text(KEY.as_ptr(), KEY.len());
        let secret = yolo_native_value_data(SECRET.as_ptr(), SECRET.len());
        let scalar = yolo_native_value_unsigned(u64::MAX);
        for (parent, name, value) in [
            (ptr::null_mut(), key, secret),
            (root, ptr::null_mut(), secret),
            (root, key, ptr::null_mut()),
            (root, root, secret),
            (root, key, root),
            (root, secret, secret),
            (scalar, key, secret),
        ] {
            assert_eq!(yolo_native_value_insert(parent, name, value), INVALID);
            assert_eq!(yolo_native_value_get_count(root), 0);
            assert_eq!(yolo_native_value_tag(secret), DATA);
            assert_eq!(yolo_native_value_tag(key), TEXT);
        }
        assert_eq!(yolo_native_value_insert(root, key, secret), OK);
        let duplicate_key = yolo_native_value_text(KEY.as_ptr(), KEY.len());
        assert_eq!(
            yolo_native_value_insert(root, duplicate_key, scalar),
            INVALID
        );
        assert_eq!(yolo_native_value_get_count(root), 1);
        assert_eq!(yolo_native_value_get_u64(scalar), u64::MAX);
        let mut length = 0;
        let bytes = yolo_native_value_get_bytes(yolo_native_value_get_value(root, 0), &mut length);
        assert_eq!(std::slice::from_raw_parts(bytes, length), SECRET);
        let bytes = yolo_native_value_get_bytes(yolo_native_value_get_key(root, 0), &mut length);
        assert_eq!(std::slice::from_raw_parts(bytes, length), KEY);
        yolo_native_value_release(duplicate_key);
        yolo_native_value_release(scalar);
        yolo_native_value_release(root);
    }
}

#[test]
fn container_capacity_and_self_insertion_fail_without_consuming_the_rejected_child() {
    unsafe {
        let list = yolo_native_value_list();
        let child = yolo_native_value_signed(i64::MIN);
        for (parent, value) in [
            (ptr::null_mut(), child),
            (list, ptr::null_mut()),
            (list, list),
        ] {
            assert_eq!(yolo_native_value_push(parent, value), INVALID);
        }
        for _ in 0..MAX_CONTAINER_ITEMS {
            assert_eq!(yolo_native_value_push(list, yolo_native_value_null()), OK);
        }
        assert_eq!(yolo_native_value_push(list, child), INVALID);
        assert_eq!(yolo_native_value_get_count(list), MAX_CONTAINER_ITEMS);
        assert_eq!(yolo_native_value_get_i64(child), i64::MIN);
        yolo_native_value_release(child);
        yolo_native_value_release(list);
    }
}

#[test]
fn invalid_encoding_never_publishes_a_partial_buffer_and_does_not_consume_the_source_tree() {
    unsafe {
        let root = yolo_native_value_list();
        let mut parent = root;
        for _ in 0..yolo_capnp_codec::MAX_VALUE_DEPTH {
            let child = yolo_native_value_list();
            assert_eq!(yolo_native_value_push(parent, child), OK);
            // The move invalidates child; obtain the current borrowed location before appending.
            parent = yolo_native_value_get_child(parent, 0).cast_mut();
        }
        assert_eq!(
            yolo_native_value_push(parent, yolo_native_value_unsigned(u64::MAX)),
            OK
        );
        let mut buffer = Buffer::default();
        assert_eq!(yolo_native_value_encode(root, &mut buffer), INVALID);
        assert!(buffer.data.is_null());
        assert_eq!(buffer.length, 0);
        assert_eq!(yolo_native_value_get_count(root), 1);
        assert_eq!(yolo_native_value_encode(ptr::null(), &mut buffer), INVALID);
        assert_eq!(yolo_native_value_encode(root, ptr::null_mut()), INVALID);
        assert_eq!(
            yolo_native_value_decode(SECRET.as_ptr(), SECRET.len(), ptr::null_mut()),
            INVALID
        );
        let mut output = ptr::null_mut();
        assert_eq!(
            yolo_native_value_decode(ptr::null(), 0, &mut output),
            INVALID
        );
        assert!(output.is_null());
        yolo_native_value_release_buffer(ptr::null_mut());
        yolo_native_value_release(root);
    }
}
