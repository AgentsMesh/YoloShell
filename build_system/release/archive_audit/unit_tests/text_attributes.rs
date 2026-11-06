use yolo_release_archive_audit::{audit_bytes, constants::*};
use yolo_release_archive_audit_test_support::*;

const PAYLOAD: &[u8] = b"hello";
const RAW_DEFLATE: &[u8] = b"\xcbH\xcd\xc9\xc9\x07\x00";
const PAYLOAD_CRC32: u32 = 0x3610_a686;
const PAYLOAD_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
const FIRST_UNSUPPORTED_BIT: u32 = 1;
const BIT_BASE: u16 = 1;
const CORRUPTED_CRC32: u32 = PAYLOAD_CRC32 ^ 1;

#[test]
fn text_hint_keeps_stored_and_deflated_payload_identity_unchanged() {
    for method in [STORED_METHOD, DEFLATE_METHOD] {
        let mut fixture = payload_archive(method);
        let binary = audit_bytes(&fixture.bytes).unwrap();
        let central = fixture.entries[PAYLOAD_INDEX].central;
        set_u16(
            &mut fixture.bytes,
            central + CENTRAL_INTERNAL_OFFSET,
            TEXT_INTERNAL_ATTRIBUTE,
        );
        let text = audit_bytes(&fixture.bytes).unwrap();
        assert_eq!(text, binary);
        let entry = &text.entries[PAYLOAD_INDEX];
        assert_eq!(entry.uncompressed_size, PAYLOAD.len() as u64);
        assert_eq!(entry.crc32, PAYLOAD_CRC32);
        assert_eq!(entry.sha256, PAYLOAD_SHA256);
        assert_eq!(entry.path.as_bytes(), PAYLOAD_PATH);
        assert_eq!(entry.mode, REGULAR_RESOURCE_MODE);
    }
}

#[test]
fn every_other_internal_attribute_bit_is_rejected_with_or_without_text_hint() {
    for bit in FIRST_UNSUPPORTED_BIT..u16::BITS {
        for text in [EMPTY_FIELD_LENGTH, TEXT_INTERNAL_ATTRIBUTE] {
            let mut fixture = payload_archive(STORED_METHOD);
            let central = fixture.entries[PAYLOAD_INDEX].central;
            set_u16(
                &mut fixture.bytes,
                central + CENTRAL_INTERNAL_OFFSET,
                text | (BIT_BASE << bit),
            );
            let error = audit_bytes(&fixture.bytes).unwrap_err();
            assert_eq!(error.to_string(), "ZIP internal attributes are unsupported");
        }
    }
}

#[test]
fn text_hint_does_not_bypass_payload_crc_or_local_central_consistency() {
    for method in [STORED_METHOD, DEFLATE_METHOD] {
        for corrupt_both_headers in [false, true] {
            let mut fixture = payload_archive(method);
            let offsets = &fixture.entries[PAYLOAD_INDEX];
            set_u16(
                &mut fixture.bytes,
                offsets.central + CENTRAL_INTERNAL_OFFSET,
                TEXT_INTERNAL_ATTRIBUTE,
            );
            set_u32(
                &mut fixture.bytes,
                offsets.central + CENTRAL_CRC_OFFSET,
                CORRUPTED_CRC32,
            );
            if corrupt_both_headers {
                set_u32(
                    &mut fixture.bytes,
                    offsets.local + LOCAL_CRC_OFFSET,
                    CORRUPTED_CRC32,
                );
            }
            assert_rejected(&fixture.bytes);
        }
    }
}

fn payload_archive(method: u16) -> ZipFixture {
    let entry = if method == DEFLATE_METHOD {
        FixtureEntry::deflated(
            PAYLOAD_PATH,
            RAW_DEFLATE,
            PAYLOAD.len() as u32,
            PAYLOAD_CRC32,
        )
    } else {
        let mut entry = FixtureEntry::regular(PAYLOAD_PATH);
        entry.payload = PAYLOAD.to_vec();
        entry.uncompressed_size = PAYLOAD.len() as u32;
        entry.crc32 = PAYLOAD_CRC32;
        entry
    };
    build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        entry,
    ])
}
