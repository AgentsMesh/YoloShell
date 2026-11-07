use crate::layout_constants::*;
use crate::layout_fixture::*;
use crate::macho;

#[test]
fn rejects_truncated_header_commands_and_mapped_file_payload() {
    let bytes = encode(&valid_image());
    for length in [
        0,
        HEADER_BYTES - 1,
        HEADER_BYTES + SEGMENT_BYTES - 1,
        HEADER_BYTES + SEGMENT_BYTES + SECTION_BYTES - 1,
        bytes.len() - 1,
    ] {
        assert!(
            validate(&bytes[..length]).is_err(),
            "accepted length {length}"
        );
    }
}

#[test]
fn rejects_fat_32_bit_and_big_endian_macho_magic() {
    for magic in [
        MAGIC_FAT,
        MAGIC_FAT.swap_bytes(),
        MAGIC_32,
        MAGIC_64.swap_bytes(),
    ] {
        let mut bytes = encode(&valid_image());
        put_u32(&mut bytes, MAGIC_OFFSET, magic);
        assert!(macho::inspect(&bytes).is_err());
    }
}

#[test]
fn rejects_segment_commands_with_missing_or_unaligned_record_bytes() {
    for size in [
        0,
        1,
        SEGMENT_BYTES as u32 - 1,
        SEGMENT_BYTES as u32 + 1,
        u32::MAX,
    ] {
        let mut bytes = encode(&valid_image());
        put_u32(&mut bytes, HEADER_BYTES + COMMAND_SIZE_OFFSET, size);
        assert!(
            macho::inspect(&bytes).is_err(),
            "accepted command size {size}"
        );
    }
}

#[test]
fn rejects_more_sections_than_fit_inside_the_load_command() {
    for count in [2, u32::MAX] {
        let mut bytes = encode(&valid_image());
        put_u32(&mut bytes, HEADER_BYTES + SECTION_COUNT_OFFSET, count);
        assert!(
            macho::inspect(&bytes).is_err(),
            "accepted section count {count}"
        );
    }
}

#[test]
fn rejects_load_command_count_exceeding_the_declared_command_region() {
    for count in [4, u32::MAX] {
        let mut bytes = encode(&valid_image());
        put_u32(&mut bytes, COMMAND_COUNT_OFFSET, count);
        assert!(
            macho::inspect(&bytes).is_err(),
            "accepted command count {count}"
        );
    }
}

#[test]
fn rejects_inconsistent_command_region_lengths() {
    for length in [0, SEGMENT_BYTES as u32 - 1, u32::MAX] {
        let mut bytes = encode(&valid_image());
        put_u32(&mut bytes, COMMAND_BYTES_OFFSET, length);
        assert!(
            macho::inspect(&bytes).is_err(),
            "accepted command bytes {length}"
        );
    }
}

#[test]
fn rejects_section_alignment_exponents_outside_u64_address_space() {
    let mut bytes = encode(&valid_image());
    put_u32(
        &mut bytes,
        HEADER_BYTES + SEGMENT_BYTES + SECTION_ALIGNMENT_OFFSET,
        u64::BITS,
    );
    assert!(validate(&bytes).is_err());
}

#[test]
fn rejects_a_section_payload_extending_beyond_its_file_mapping() {
    let mut image = valid_image();
    image.segments[COUNTERS].sections[FIRST_SECTION].size = FILE_BYTES;
    assert_rejected(&image);
}
