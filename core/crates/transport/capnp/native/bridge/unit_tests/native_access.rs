use super::*;

const UTF8: &[u8] = "目录\0title".as_bytes();
const BINARY: &[u8] = b"\0\xff\n\0";
const PRECISE_FLOAT: f64 = -1.5;
const LENGTH_SENTINEL: usize = 99;

#[test]
fn native_reader_preserves_scalar_types_and_borrowed_utf8_and_binary_bytes() {
    unsafe {
        let root = yolo_native_value_list();
        let children = [
            (yolo_native_value_null(), NULL),
            (yolo_native_value_boolean(true), BOOLEAN),
            (yolo_native_value_signed(i64::MIN), SIGNED),
            (yolo_native_value_unsigned(u64::MAX), UNSIGNED),
            (yolo_native_value_float(PRECISE_FLOAT), FLOAT),
            (yolo_native_value_text(UTF8.as_ptr(), UTF8.len()), TEXT),
            (yolo_native_value_data(BINARY.as_ptr(), BINARY.len()), DATA),
            (yolo_native_value_map(), MAP),
        ];
        for (child, _) in children {
            assert_eq!(yolo_native_value_push(root, child), OK);
        }
        assert_eq!(yolo_native_value_tag(root), LIST);
        assert_eq!(yolo_native_value_get_count(root), children.len());
        for (index, (_, tag)) in children.iter().enumerate() {
            let child = yolo_native_value_get_child(root, index);
            assert_eq!(yolo_native_value_tag(child), *tag);
            match *tag {
                BOOLEAN => assert!(yolo_native_value_get_bool(child)),
                SIGNED => assert_eq!(yolo_native_value_get_i64(child), i64::MIN),
                UNSIGNED => assert_eq!(yolo_native_value_get_u64(child), u64::MAX),
                FLOAT => assert_eq!(yolo_native_value_get_f64(child), PRECISE_FLOAT),
                TEXT | DATA => {
                    let mut length = LENGTH_SENTINEL;
                    let bytes = yolo_native_value_get_bytes(child, &mut length);
                    let expected = if *tag == TEXT { UTF8 } else { BINARY };
                    assert_eq!(std::slice::from_raw_parts(bytes, length), expected);
                    assert!(yolo_native_value_get_bytes(child, ptr::null_mut()).is_null());
                }
                _ => assert_eq!(yolo_native_value_get_count(child), 0),
            }
        }
        assert!(yolo_native_value_get_child(root, children.len()).is_null());
        assert!(yolo_native_value_get_child(root, usize::MAX).is_null());
        yolo_native_value_release(root);
    }
}

#[test]
fn missing_or_wrong_kind_accessors_never_dereference_children_or_leave_stale_lengths() {
    unsafe {
        let scalar = yolo_native_value_null();
        for root in [ptr::null(), scalar as *const Value] {
            assert_eq!(yolo_native_value_get_i64(root), 0);
            assert_eq!(yolo_native_value_get_u64(root), 0);
            assert_eq!(yolo_native_value_get_f64(root), 0.0);
            assert!(!yolo_native_value_get_bool(root));
            assert_eq!(yolo_native_value_get_count(root), 0);
            assert!(yolo_native_value_get_child(root, 0).is_null());
            assert!(yolo_native_value_get_key(root, 0).is_null());
            assert!(yolo_native_value_get_value(root, 0).is_null());
            let mut length = LENGTH_SENTINEL;
            assert!(yolo_native_value_get_bytes(root, &mut length).is_null());
            assert_eq!(length, 0);
        }
        assert_eq!(yolo_native_value_tag(ptr::null()), INVALID_TAG);
        let empty = yolo_native_value_map();
        assert!(yolo_native_value_get_key(empty, 0).is_null());
        assert!(yolo_native_value_get_value(empty, usize::MAX).is_null());
        for constructor in [yolo_native_value_text, yolo_native_value_data] {
            let bytes = constructor(ptr::null(), 0);
            assert!(!bytes.is_null());
            let mut length = LENGTH_SENTINEL;
            yolo_native_value_get_bytes(bytes, &mut length);
            assert_eq!(length, 0);
            assert!(constructor(ptr::null(), 1).is_null());
            yolo_native_value_release(bytes);
        }
        yolo_native_value_release(empty);
        yolo_native_value_release(scalar);
    }
}
