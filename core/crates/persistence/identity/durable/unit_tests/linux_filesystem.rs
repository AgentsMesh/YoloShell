use super::*;

const UUID_BYTE: u8 = 0x71;

#[test]
fn kernel_uuid_length_must_describe_the_entire_fixed_identity() {
    for length in [
        u8::default(),
        UUID_BYTES as u8 - 1,
        UUID_BYTES as u8 + 1,
        u8::MAX,
    ] {
        assert!(matches!(
            parse_uuid(FilesystemUuid {
                length,
                uuid: [UUID_BYTE; UUID_BYTES]
            }),
            Err(Error::Unsupported(_))
        ));
    }
    assert_eq!(
        parse_uuid(FilesystemUuid {
            length: UUID_BYTES as u8,
            uuid: [UUID_BYTE; UUID_BYTES],
        })
        .unwrap(),
        [UUID_BYTE; UUID_BYTES]
    );
}
