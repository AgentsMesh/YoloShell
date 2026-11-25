use crate::constants::*;
use crate::layout_constants::*;
use crate::layout_fixture::*;
use crate::macho;

#[test]
fn accepts_empty_bitmap_with_zero_section_file_offset() {
    let mut image = valid_image();
    let bitmap = &mut image.segments[BITMAP].sections[FIRST_SECTION];
    bitmap.size = 0;
    bitmap.file_offset = 0;
    validate(&encode(&image)).unwrap();
}

#[test]
fn accepts_empty_bitmap_in_an_empty_dedicated_segment() {
    let mut image = valid_image();
    let segment = &mut image.segments[BITMAP];
    segment.size = 0;
    segment.file_size = 0;
    segment.file_offset = 0;
    let bitmap = &mut segment.sections[FIRST_SECTION];
    bitmap.size = 0;
    bitmap.file_offset = 0;
    bitmap.address = segment.address;
    validate(&encode(&image)).unwrap();
}

#[test]
fn rejects_32_bit_segment_commands_inside_a_64_bit_image() {
    let mut bytes = encode(&valid_image());
    put_u32(&mut bytes, HEADER_BYTES, SEGMENT_32);
    assert!(macho::inspect(&bytes).is_err());
}

#[test]
fn rejects_a_header_without_any_load_commands() {
    let mut bytes = encode(&valid_image());
    put_u32(&mut bytes, COMMAND_COUNT_OFFSET, 0);
    put_u32(&mut bytes, COMMAND_BYTES_OFFSET, 0);
    assert!(macho::inspect(&bytes).is_err());
}

#[test]
fn rejects_invalid_names_and_nonzero_bytes_after_name_termination() {
    for offset in [
        HEADER_BYTES + SEGMENT_NAME_OFFSET,
        HEADER_BYTES + SEGMENT_BYTES,
        HEADER_BYTES + SEGMENT_BYTES + SECTION_SEGMENT_OFFSET,
    ] {
        for invalid in [&b"bad name"[..], &b"A\0B"[..], &b"\xff"[..], &b""[..]] {
            let mut bytes = encode(&valid_image());
            bytes[offset..offset + NAME_BYTES].fill(0);
            bytes[offset..offset + invalid.len()].copy_from_slice(invalid);
            assert!(macho::inspect(&bytes).is_err(), "accepted name {invalid:?}");
        }
    }
}

#[test]
fn rejects_duplicate_profile_section_names_in_otherwise_valid_segments() {
    let mut image = valid_image();
    let mut duplicate = valid_image().segments.remove(COUNTERS);
    duplicate.name = NEIGHBOR_SEGMENT.into();
    duplicate.address = BASE_ADDRESS + FILE_BYTES;
    duplicate.file_offset = FILE_BYTES;
    let section = &mut duplicate.sections[FIRST_SECTION];
    section.segment = NEIGHBOR_SEGMENT.into();
    section.address = duplicate.address;
    section.file_offset = duplicate.file_offset;
    image.segments.push(duplicate);
    let mut bytes = encode(&image);
    bytes.resize((FILE_BYTES + PROFILE_PAGE_BYTES) as usize, 0);
    let parsed = macho::inspect(&bytes).unwrap();
    assert!(macho::verify_profile_layout(&parsed).is_err());
}
