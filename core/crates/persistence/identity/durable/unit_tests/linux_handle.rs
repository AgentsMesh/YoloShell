use super::*;

const VALID_KIND: i32 = 77;
const INVALID_KIND: i32 = 255;
const BYTE: u8 = 0x51;

fn response(length: usize, kind: i32) -> NativeHandle {
    NativeHandle {
        header: libc::file_handle {
            handle_bytes: length as u32,
            handle_type: kind,
            f_handle: [],
        },
        bytes: [BYTE; HANDLE_CAPACITY],
    }
}

#[test]
fn kernel_handle_cannot_claim_bytes_outside_the_supplied_buffer() {
    for length in [usize::default(), HANDLE_CAPACITY + 1] {
        assert!(matches!(
            parse_handle(&response(length, VALID_KIND)),
            Err(Error::Invalid(_))
        ));
    }
    for kind in [i32::default(), -1, INVALID_KIND] {
        assert!(matches!(
            parse_handle(&response(HANDLE_CAPACITY, kind)),
            Err(Error::Invalid(_))
        ));
    }
    assert_eq!(
        parse_handle(&response(HANDLE_CAPACITY, VALID_KIND)).unwrap(),
        Handle::new(VALID_KIND, &[BYTE; HANDLE_CAPACITY]).unwrap()
    );
}
