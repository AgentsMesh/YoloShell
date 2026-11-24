use yolo_release_contracts::*;
#[path = "fixture_corruption.rs"]
pub(crate) mod corruption;
#[path = "fixture_dynamic_boundaries.rs"]
pub(crate) mod dynamic_boundaries;
#[path = "fixture_write.rs"]
mod write;
use write::{write_i64, write_u16, write_u32, write_u64};

const ELF_VERSION_OFFSET: usize = 6;
const ELF_CURRENT_VERSION: u8 = 1;
const ELF_HEADER_SIZE_OFFSET: usize = 52;
const ELF_PROGRAM_HEADER_ENTRY_SIZE_OFFSET: usize = 54;
const ELF_PROGRAM_HEADER_COUNT_OFFSET: usize = 56;
const ELF_PROGRAM_HEADER_TABLE_OFFSET: usize = 32;
const ELF64_COMPLETE_HEADER_BYTES_U16: u16 = ELF64_COMPLETE_HEADER_BYTES as u16;
const ELF64_COMPLETE_HEADER_BYTES_U64: u64 = ELF64_COMPLETE_HEADER_BYTES as u64;
const PROGRAM_HEADER_BYTES: usize = 56;
const PROGRAM_HEADER_COUNT: u16 = 2;
const PROGRAM_TYPE_OFFSET: usize = 0;
const PROGRAM_FILE_OFFSET: usize = 8;
const PROGRAM_VIRTUAL_ADDRESS_OFFSET: usize = 16;
const PROGRAM_FILE_SIZE_OFFSET: usize = 32;
const PROGRAM_LOAD_TYPE: u32 = 1;
const PROGRAM_DYNAMIC_TYPE: u32 = 2;
const DYNAMIC_ENTRY_BYTES: usize = 16;
const DYNAMIC_ENTRY_COUNT: usize = 5;
const DYNAMIC_STRING_SIZE_ENTRY_INDEX: usize = 1;
const DYNAMIC_VERSION_NEED_ENTRY_INDEX: usize = 2;
const DYNAMIC_TABLE_OFFSET: usize = ELF64_COMPLETE_HEADER_BYTES + PROGRAM_HEADER_BYTES * 2;
const DYNAMIC_TABLE_BYTES: usize = DYNAMIC_ENTRY_BYTES * DYNAMIC_ENTRY_COUNT;
const DYNAMIC_TAG_OFFSET: usize = 0;
const DYNAMIC_VALUE_OFFSET: usize = 8;
const DYNAMIC_NULL_TAG: i64 = 0;
const DYNAMIC_STRING_TABLE_TAG: i64 = 5;
const DYNAMIC_STRING_SIZE_TAG: i64 = 10;
const DYNAMIC_VERSION_NEED_TAG: i64 = 0x6fff_fffe;
const DYNAMIC_VERSION_NEED_COUNT_TAG: i64 = 0x6fff_ffff;
const STRING_TABLE_OFFSET: usize = DYNAMIC_TABLE_OFFSET + DYNAMIC_TABLE_BYTES;
const VERSION_NEED_OFFSET: usize = 512;
const VERSION_NEED_BYTES: usize = 16;
const VERSION_NEED_REVISION_OFFSET: usize = 0;
const VERSION_NEED_COUNT_OFFSET: usize = 2;
const VERSION_NEED_AUXILIARY_OFFSET: usize = 8;
const VERSION_NEED_NEXT_OFFSET: usize = 12;
const VERSION_NEED_CURRENT_REVISION: u16 = 1;
const VERSION_AUXILIARY_BYTES: usize = 16;
const VERSION_AUXILIARY_NAME_OFFSET: usize = 8;
const VERSION_AUXILIARY_NEXT_OFFSET: usize = 12;
const TEST_VIRTUAL_BASE: u64 = 0x40_0000;
const STRING_TERMINATOR: u8 = 0;

pub(crate) fn static_elf(machine: u16) -> Vec<u8> {
    let mut bytes = vec![0; ELF64_COMPLETE_HEADER_BYTES];
    write_header(&mut bytes, machine, 0);
    bytes
}

pub(crate) fn versioned_elf(machine: u16, versions: &[&str]) -> Vec<u8> {
    assert!(!versions.is_empty());
    let file_size =
        VERSION_NEED_OFFSET + VERSION_NEED_BYTES + versions.len() * VERSION_AUXILIARY_BYTES;
    let mut bytes = vec![0; file_size];
    write_header(&mut bytes, machine, PROGRAM_HEADER_COUNT);
    let mut string_cursor = STRING_TABLE_OFFSET + 1;
    let mut name_offsets = Vec::with_capacity(versions.len());
    for version in versions {
        let name_offset = string_cursor - STRING_TABLE_OFFSET;
        name_offsets.push(u32::try_from(name_offset).unwrap());
        let end = string_cursor + version.len();
        bytes[string_cursor..end].copy_from_slice(version.as_bytes());
        bytes[end] = STRING_TERMINATOR;
        string_cursor = end + 1;
    }
    assert!(string_cursor <= VERSION_NEED_OFFSET);
    write_program_headers(&mut bytes);
    write_dynamic_entries(&mut bytes, string_cursor - STRING_TABLE_OFFSET);
    write_version_needs(&mut bytes, &name_offsets);
    bytes
}

pub(crate) fn remove_version_metadata(bytes: &mut [u8]) {
    let entry = DYNAMIC_TABLE_OFFSET + DYNAMIC_VERSION_NEED_ENTRY_INDEX * DYNAMIC_ENTRY_BYTES;
    write_i64(bytes, entry + DYNAMIC_TAG_OFFSET, DYNAMIC_NULL_TAG);
}

pub(crate) fn overflow_string_table(bytes: &mut [u8]) {
    let entry = DYNAMIC_TABLE_OFFSET + DYNAMIC_STRING_SIZE_ENTRY_INDEX * DYNAMIC_ENTRY_BYTES;
    write_u64(bytes, entry + DYNAMIC_VALUE_OFFSET, u64::MAX);
}

fn write_header(bytes: &mut [u8], machine: u16, program_count: u16) {
    bytes[..ELF_MAGIC.len()].copy_from_slice(ELF_MAGIC);
    bytes[ELF_CLASS_OFFSET] = ELF_64_CLASS;
    bytes[ELF_DATA_OFFSET] = ELF_LITTLE_ENDIAN_DATA;
    bytes[ELF_VERSION_OFFSET] = ELF_CURRENT_VERSION;
    write_u16(bytes, ELF_MACHINE_OFFSET, machine);
    write_u64(
        bytes,
        ELF_PROGRAM_HEADER_TABLE_OFFSET,
        ELF64_COMPLETE_HEADER_BYTES_U64,
    );
    write_u16(
        bytes,
        ELF_HEADER_SIZE_OFFSET,
        ELF64_COMPLETE_HEADER_BYTES_U16,
    );
    write_u16(
        bytes,
        ELF_PROGRAM_HEADER_ENTRY_SIZE_OFFSET,
        PROGRAM_HEADER_BYTES as u16,
    );
    write_u16(bytes, ELF_PROGRAM_HEADER_COUNT_OFFSET, program_count);
}

fn write_program_headers(bytes: &mut [u8]) {
    let load = ELF64_COMPLETE_HEADER_BYTES;
    write_u32(bytes, load + PROGRAM_TYPE_OFFSET, PROGRAM_LOAD_TYPE);
    write_u64(bytes, load + PROGRAM_FILE_OFFSET, 0);
    write_u64(
        bytes,
        load + PROGRAM_VIRTUAL_ADDRESS_OFFSET,
        TEST_VIRTUAL_BASE,
    );
    write_u64(bytes, load + PROGRAM_FILE_SIZE_OFFSET, bytes.len() as u64);

    let dynamic = load + PROGRAM_HEADER_BYTES;
    write_u32(bytes, dynamic + PROGRAM_TYPE_OFFSET, PROGRAM_DYNAMIC_TYPE);
    write_u64(
        bytes,
        dynamic + PROGRAM_FILE_OFFSET,
        DYNAMIC_TABLE_OFFSET as u64,
    );
    write_u64(
        bytes,
        dynamic + PROGRAM_VIRTUAL_ADDRESS_OFFSET,
        TEST_VIRTUAL_BASE + DYNAMIC_TABLE_OFFSET as u64,
    );
    write_u64(
        bytes,
        dynamic + PROGRAM_FILE_SIZE_OFFSET,
        DYNAMIC_TABLE_BYTES as u64,
    );
}

fn write_dynamic_entries(bytes: &mut [u8], string_size: usize) {
    let values = [
        (
            DYNAMIC_STRING_TABLE_TAG,
            TEST_VIRTUAL_BASE + STRING_TABLE_OFFSET as u64,
        ),
        (DYNAMIC_STRING_SIZE_TAG, string_size as u64),
        (
            DYNAMIC_VERSION_NEED_TAG,
            TEST_VIRTUAL_BASE + VERSION_NEED_OFFSET as u64,
        ),
        (DYNAMIC_VERSION_NEED_COUNT_TAG, 1),
        (DYNAMIC_NULL_TAG, 0),
    ];
    for (index, (tag, value)) in values.into_iter().enumerate() {
        let entry = DYNAMIC_TABLE_OFFSET + index * DYNAMIC_ENTRY_BYTES;
        write_i64(bytes, entry + DYNAMIC_TAG_OFFSET, tag);
        write_u64(bytes, entry + DYNAMIC_VALUE_OFFSET, value);
    }
}

fn write_version_needs(bytes: &mut [u8], name_offsets: &[u32]) {
    write_u16(
        bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_REVISION_OFFSET,
        VERSION_NEED_CURRENT_REVISION,
    );
    write_u16(
        bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_COUNT_OFFSET,
        name_offsets.len() as u16,
    );
    write_u32(
        bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_AUXILIARY_OFFSET,
        VERSION_NEED_BYTES as u32,
    );
    write_u32(bytes, VERSION_NEED_OFFSET + VERSION_NEED_NEXT_OFFSET, 0);
    for (index, name_offset) in name_offsets.iter().copied().enumerate() {
        let auxiliary = VERSION_NEED_OFFSET + VERSION_NEED_BYTES + index * VERSION_AUXILIARY_BYTES;
        write_u32(
            bytes,
            auxiliary + VERSION_AUXILIARY_NAME_OFFSET,
            name_offset,
        );
        let next = if index + 1 == name_offsets.len() {
            0
        } else {
            VERSION_AUXILIARY_BYTES as u32
        };
        write_u32(bytes, auxiliary + VERSION_AUXILIARY_NEXT_OFFSET, next);
    }
}
