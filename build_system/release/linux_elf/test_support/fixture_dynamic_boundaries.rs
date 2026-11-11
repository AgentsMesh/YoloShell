use super::*;

const UNKNOWN_DYNAMIC_TAG: i64 = i64::MAX;
const VERSION_COUNT_INDEX: usize = 3;
const REQUIRED_DYNAMIC_VALUES: usize = 4;
const TOO_MANY_DYNAMIC_ENTRIES: usize = 4_097;
const FIRST_STRING_OFFSET: usize = 1;
const INVALID_UTF8: u8 = 0xff;

pub(crate) fn missing_values() -> Vec<Vec<u8>> {
    (0..REQUIRED_DYNAMIC_VALUES)
        .map(|index| {
            let mut bytes = valid();
            write_i64(
                &mut bytes,
                DYNAMIC_TABLE_OFFSET + index * DYNAMIC_ENTRY_BYTES,
                UNKNOWN_DYNAMIC_TAG,
            );
            bytes
        })
        .collect()
}

pub(crate) fn invalid_sizes() -> Vec<Vec<u8>> {
    let mut malformed = valid();
    write_u64(
        &mut malformed,
        dynamic_header() + PROGRAM_FILE_SIZE_OFFSET,
        (DYNAMIC_TABLE_BYTES - 1) as u64,
    );
    let mut oversized = valid();
    let table_size = TOO_MANY_DYNAMIC_ENTRIES * DYNAMIC_ENTRY_BYTES;
    oversized.resize(DYNAMIC_TABLE_OFFSET + table_size, 0);
    write_u64(
        &mut oversized,
        dynamic_header() + PROGRAM_FILE_SIZE_OFFSET,
        table_size as u64,
    );
    let length = oversized.len() as u64;
    write_u64(
        &mut oversized,
        ELF64_COMPLETE_HEADER_BYTES + PROGRAM_FILE_SIZE_OFFSET,
        length,
    );
    vec![malformed, oversized]
}

pub(crate) fn duplicate_segment() -> Vec<u8> {
    let mut bytes = valid();
    write_u32(
        &mut bytes,
        ELF64_COMPLETE_HEADER_BYTES + PROGRAM_TYPE_OFFSET,
        PROGRAM_DYNAMIC_TYPE,
    );
    bytes
}

pub(crate) fn empty_required_tables() -> Vec<Vec<u8>> {
    [DYNAMIC_STRING_SIZE_ENTRY_INDEX, VERSION_COUNT_INDEX]
        .into_iter()
        .map(|index| {
            let mut bytes = valid();
            write_u64(
                &mut bytes,
                DYNAMIC_TABLE_OFFSET + index * DYNAMIC_ENTRY_BYTES + DYNAMIC_VALUE_OFFSET,
                0,
            );
            bytes
        })
        .collect()
}

pub(crate) fn unmapped_version_table() -> Vec<u8> {
    let mut bytes = valid();
    write_u64(
        &mut bytes,
        DYNAMIC_TABLE_OFFSET
            + DYNAMIC_VERSION_NEED_ENTRY_INDEX * DYNAMIC_ENTRY_BYTES
            + DYNAMIC_VALUE_OFFSET,
        u64::MAX,
    );
    bytes
}

pub(crate) fn invalid_version_encoding() -> Vec<u8> {
    let mut bytes = valid();
    bytes[STRING_TABLE_OFFSET + FIRST_STRING_OFFSET + GLIBC_SYMBOL_VERSION_PREFIX.len()] =
        INVALID_UTF8;
    bytes
}

pub(crate) fn two_version_records() -> Vec<u8> {
    let mut bytes = valid();
    let delta = VERSION_NEED_BYTES + VERSION_AUXILIARY_BYTES;
    let second = VERSION_NEED_OFFSET + delta;
    bytes.resize(second + delta, 0);
    let length = bytes.len() as u64;
    write_u64(
        &mut bytes,
        ELF64_COMPLETE_HEADER_BYTES + PROGRAM_FILE_SIZE_OFFSET,
        length,
    );
    write_u64(
        &mut bytes,
        DYNAMIC_TABLE_OFFSET + VERSION_COUNT_INDEX * DYNAMIC_ENTRY_BYTES + DYNAMIC_VALUE_OFFSET,
        2,
    );
    write_u32(
        &mut bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_NEXT_OFFSET,
        delta as u32,
    );
    write_u16(
        &mut bytes,
        second + VERSION_NEED_REVISION_OFFSET,
        VERSION_NEED_CURRENT_REVISION,
    );
    write_u16(&mut bytes, second + VERSION_NEED_COUNT_OFFSET, 1);
    write_u32(
        &mut bytes,
        second + VERSION_NEED_AUXILIARY_OFFSET,
        VERSION_NEED_BYTES as u32,
    );
    write_u32(
        &mut bytes,
        second + VERSION_NEED_BYTES + VERSION_AUXILIARY_NAME_OFFSET,
        FIRST_STRING_OFFSET as u32,
    );
    bytes
}

fn valid() -> Vec<u8> {
    let version = format!("{GLIBC_SYMBOL_VERSION_PREFIX}{LINUX_MINIMUM_GLIBC_VERSION}");
    versioned_elf(ELF_MACHINE_AARCH64, &[&version])
}

fn dynamic_header() -> usize {
    ELF64_COMPLETE_HEADER_BYTES + PROGRAM_HEADER_BYTES
}
