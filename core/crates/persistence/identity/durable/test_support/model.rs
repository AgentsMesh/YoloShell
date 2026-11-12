use crate::model::{
    BTRFS_MAGIC, Birth, DurableObjectIdentity, Handle, ObjectKind, Proof, UUID_BYTES,
};

const TEST_UUID_BYTE: u8 = 0x71;
const TEST_HANDLE_TYPE: i32 = 77;
const TEST_HANDLE_BYTES: &[u8] = b"bounded-native-handle";
const TEST_BIRTH_SECONDS: i64 = 1_700_000_000;
const TEST_FILE_ID: u64 = 251;

pub(super) fn linux() -> DurableObjectIdentity {
    DurableObjectIdentity::new(
        ObjectKind::Directory,
        Birth {
            seconds: TEST_BIRTH_SECONDS,
            nanoseconds: u32::default(),
        },
        Proof::Linux {
            filesystem_type: BTRFS_MAGIC,
            filesystem_uuid: [TEST_UUID_BYTE; UUID_BYTES],
            handle: Handle::new(TEST_HANDLE_TYPE, TEST_HANDLE_BYTES).unwrap(),
        },
    )
    .unwrap()
}

pub(super) fn macos() -> DurableObjectIdentity {
    DurableObjectIdentity::new(
        ObjectKind::RegularFile,
        Birth {
            seconds: TEST_BIRTH_SECONDS,
            nanoseconds: u32::default(),
        },
        Proof::Macos {
            volume_uuid: [TEST_UUID_BYTE; UUID_BYTES],
            file_id: TEST_FILE_ID,
        },
    )
    .unwrap()
}
