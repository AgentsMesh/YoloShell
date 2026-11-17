use yolo_release_archive_audit::constants::*;

const CREATOR_VERSION: u16 = 30;
pub const ROOT_INDEX: usize = 0;
pub const PAYLOAD_INDEX: usize = 1;
pub const PAYLOAD_PATH: &[u8] = b"Yolo.app/Contents/payload";

#[derive(Clone)]
pub struct FixtureEntry {
    pub central_name: Vec<u8>,
    pub local_name: Vec<u8>,
    pub unix_mode: u16,
    pub origin: u8,
    pub payload: Vec<u8>,
    pub method: u16,
    pub crc32: u32,
    pub uncompressed_size: u32,
}

impl FixtureEntry {
    pub fn directory(name: &[u8]) -> Self {
        Self {
            central_name: name.to_vec(),
            local_name: name.to_vec(),
            unix_mode: UNIX_DIRECTORY_TYPE | DIRECTORY_MODE,
            origin: UNIX_ORIGIN,
            payload: Vec::new(),
            method: STORED_METHOD,
            crc32: EMPTY_CRC32,
            uncompressed_size: EMPTY_ENTRY_BYTES as u32,
        }
    }

    pub fn regular(name: &[u8]) -> Self {
        Self {
            central_name: name.to_vec(),
            local_name: name.to_vec(),
            unix_mode: UNIX_REGULAR_TYPE | REGULAR_RESOURCE_MODE,
            origin: UNIX_ORIGIN,
            payload: Vec::new(),
            method: STORED_METHOD,
            crc32: EMPTY_CRC32,
            uncompressed_size: EMPTY_ENTRY_BYTES as u32,
        }
    }

    pub fn deflated(name: &[u8], payload: &[u8], size: u32, crc32: u32) -> Self {
        let mut entry = Self::regular(name);
        entry.payload = payload.to_vec();
        entry.method = DEFLATE_METHOD;
        entry.crc32 = crc32;
        entry.uncompressed_size = size;
        entry
    }
}

pub struct EntryOffsets {
    pub local: usize,
    pub local_name: usize,
    pub central: usize,
    pub central_name: usize,
}

pub struct ZipFixture {
    pub bytes: Vec<u8>,
    pub entries: Vec<EntryOffsets>,
    pub central_offset: usize,
    pub eocd_offset: usize,
}

pub fn valid_archive() -> ZipFixture {
    build(&[
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::regular(PAYLOAD_PATH),
    ])
}

pub fn assert_rejected(bytes: &[u8]) {
    assert!(yolo_release_archive_audit::audit_bytes(bytes).is_err());
}

pub fn assert_accepted(bytes: &[u8]) {
    yolo_release_archive_audit::audit_bytes(bytes).unwrap();
}

pub fn build(entries: &[FixtureEntry]) -> ZipFixture {
    let mut bytes = Vec::new();
    let mut offsets = Vec::with_capacity(entries.len());
    for entry in entries {
        let local = bytes.len();
        append_u32(&mut bytes, LOCAL_SIGNATURE);
        append_u16(&mut bytes, MIN_VERSION_NEEDED);
        append_u16(&mut bytes, SUPPORTED_FLAGS);
        append_u16(&mut bytes, entry.method);
        append_u16(&mut bytes, 0);
        append_u16(&mut bytes, 0);
        append_u32(&mut bytes, entry.crc32);
        append_u32(&mut bytes, entry.payload.len() as u32);
        append_u32(&mut bytes, entry.uncompressed_size);
        append_u16(&mut bytes, entry.local_name.len() as u16);
        append_u16(&mut bytes, 0);
        let local_name = bytes.len();
        bytes.extend_from_slice(&entry.local_name);
        bytes.extend_from_slice(&entry.payload);
        offsets.push(EntryOffsets {
            local,
            local_name,
            central: 0,
            central_name: 0,
        });
    }
    let central_offset = bytes.len();
    for (index, entry) in entries.iter().enumerate() {
        offsets[index].central = bytes.len();
        append_u32(&mut bytes, CENTRAL_SIGNATURE);
        append_u16(
            &mut bytes,
            u16::from(entry.origin) << u8::BITS | CREATOR_VERSION,
        );
        append_u16(&mut bytes, MIN_VERSION_NEEDED);
        append_u16(&mut bytes, SUPPORTED_FLAGS);
        append_u16(&mut bytes, entry.method);
        append_u16(&mut bytes, 0);
        append_u16(&mut bytes, 0);
        append_u32(&mut bytes, entry.crc32);
        append_u32(&mut bytes, entry.payload.len() as u32);
        append_u32(&mut bytes, entry.uncompressed_size);
        append_u16(&mut bytes, entry.central_name.len() as u16);
        append_u16(&mut bytes, 0);
        append_u16(&mut bytes, 0);
        append_u16(&mut bytes, 0);
        append_u16(&mut bytes, 0);
        let dos_attributes = if entry.unix_mode & UNIX_TYPE_MASK == UNIX_DIRECTORY_TYPE {
            u32::from(DOS_DIRECTORY_ATTRIBUTE)
        } else {
            0
        };
        append_u32(
            &mut bytes,
            u32::from(entry.unix_mode) << u16::BITS | dos_attributes,
        );
        append_u32(&mut bytes, offsets[index].local as u32);
        offsets[index].central_name = bytes.len();
        bytes.extend_from_slice(&entry.central_name);
    }
    let eocd_offset = bytes.len();
    append_u32(&mut bytes, EOCD_SIGNATURE);
    append_u16(&mut bytes, 0);
    append_u16(&mut bytes, 0);
    append_u16(&mut bytes, entries.len() as u16);
    append_u16(&mut bytes, entries.len() as u16);
    append_u32(&mut bytes, (eocd_offset - central_offset) as u32);
    append_u32(&mut bytes, central_offset as u32);
    append_u16(&mut bytes, 0);
    ZipFixture {
        bytes,
        entries: offsets,
        central_offset,
        eocd_offset,
    }
}

pub fn set_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

pub fn set_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn append_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn append_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
