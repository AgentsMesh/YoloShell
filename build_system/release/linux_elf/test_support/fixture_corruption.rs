use super::*;

const EXCESSIVE_PROGRAM_HEADERS: u16 = 4_097;
const EXCESSIVE_VERSION_NEEDS: u64 = 1_025;
const EXCESSIVE_AUXILIARIES: u16 = 4_097;
const VERSION_NEED_COUNT_ENTRY_INDEX: usize = 3;
const LAST_DYNAMIC_ENTRY_INDEX: usize = DYNAMIC_ENTRY_COUNT - 1;
const SHORT_RECORD_OFFSET: u32 = 1;
const INVALID_REVISION: u16 = 2;
const UNMAPPED_ADDRESS: u64 = u64::MAX;

pub fn invalid_program_entry_size(bytes: &mut [u8]) {
    write_u16(bytes, ELF_PROGRAM_HEADER_ENTRY_SIZE_OFFSET, 0);
}

pub fn too_many_program_headers(bytes: &mut [u8]) {
    write_u16(
        bytes,
        ELF_PROGRAM_HEADER_COUNT_OFFSET,
        EXCESSIVE_PROGRAM_HEADERS,
    );
}

pub fn string_address_before_load_segment(bytes: &mut [u8]) {
    write_u64(
        bytes,
        DYNAMIC_TABLE_OFFSET + DYNAMIC_VALUE_OFFSET,
        TEST_VIRTUAL_BASE - 1,
    );
}

pub fn string_address_beyond_load_segment(bytes: &mut [u8]) {
    write_u64(
        bytes,
        DYNAMIC_TABLE_OFFSET + DYNAMIC_VALUE_OFFSET,
        UNMAPPED_ADDRESS,
    );
}

pub fn too_many_version_needs(bytes: &mut [u8]) {
    let entry = DYNAMIC_TABLE_OFFSET + VERSION_NEED_COUNT_ENTRY_INDEX * DYNAMIC_ENTRY_BYTES;
    write_u64(bytes, entry + DYNAMIC_VALUE_OFFSET, EXCESSIVE_VERSION_NEEDS);
}

pub fn invalid_version_revision(bytes: &mut [u8]) {
    write_u16(
        bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_REVISION_OFFSET,
        INVALID_REVISION,
    );
}

pub fn empty_auxiliary_list(bytes: &mut [u8]) {
    write_u16(bytes, VERSION_NEED_OFFSET + VERSION_NEED_COUNT_OFFSET, 0);
}

pub fn too_many_auxiliaries(bytes: &mut [u8]) {
    write_u16(
        bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_COUNT_OFFSET,
        EXCESSIVE_AUXILIARIES,
    );
}

pub fn truncated_auxiliary_pointer(bytes: &mut [u8]) {
    write_u32(
        bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_AUXILIARY_OFFSET,
        u32::MAX,
    );
}

pub fn truncated_version_name_pointer(bytes: &mut [u8]) {
    let auxiliary = VERSION_NEED_OFFSET + VERSION_NEED_BYTES;
    write_u32(bytes, auxiliary + VERSION_AUXILIARY_NAME_OFFSET, u32::MAX);
}

pub fn unterminated_version_name(bytes: &mut [u8]) {
    let entry = DYNAMIC_TABLE_OFFSET + DYNAMIC_STRING_SIZE_ENTRY_INDEX * DYNAMIC_ENTRY_BYTES;
    let field = entry + DYNAMIC_VALUE_OFFSET;
    let size = u64::from_le_bytes(
        bytes[field..field + std::mem::size_of::<u64>()]
            .try_into()
            .unwrap(),
    );
    write_u64(bytes, field, size - 1);
}

pub fn nonterminal_last_version_record(bytes: &mut [u8]) {
    write_u32(
        bytes,
        VERSION_NEED_OFFSET + VERSION_NEED_NEXT_OFFSET,
        VERSION_NEED_BYTES as u32,
    );
}

pub fn nonterminal_last_auxiliary_record(bytes: &mut [u8]) {
    let auxiliary = VERSION_NEED_OFFSET + VERSION_NEED_BYTES;
    write_u32(
        bytes,
        auxiliary + VERSION_AUXILIARY_NEXT_OFFSET,
        VERSION_AUXILIARY_BYTES as u32,
    );
}

pub fn overlapping_auxiliary_records(bytes: &mut [u8]) {
    let auxiliary = VERSION_NEED_OFFSET + VERSION_NEED_BYTES;
    write_u16(bytes, VERSION_NEED_OFFSET + VERSION_NEED_COUNT_OFFSET, 2);
    write_u32(
        bytes,
        auxiliary + VERSION_AUXILIARY_NEXT_OFFSET,
        SHORT_RECORD_OFFSET,
    );
}

pub fn duplicate_dynamic_value(bytes: &mut [u8]) {
    let entry = DYNAMIC_TABLE_OFFSET + DYNAMIC_STRING_SIZE_ENTRY_INDEX * DYNAMIC_ENTRY_BYTES;
    write_i64(bytes, entry + DYNAMIC_TAG_OFFSET, DYNAMIC_STRING_TABLE_TAG);
}

pub fn unterminated_dynamic_table(bytes: &mut [u8]) {
    let entry = DYNAMIC_TABLE_OFFSET + LAST_DYNAMIC_ENTRY_INDEX * DYNAMIC_ENTRY_BYTES;
    write_i64(bytes, entry + DYNAMIC_TAG_OFFSET, i64::MAX);
}
