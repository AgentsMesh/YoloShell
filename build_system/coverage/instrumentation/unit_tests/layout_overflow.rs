use crate::constants::PROFILE_PAGE_BYTES;
use crate::layout_constants::*;
use crate::layout_fixture::*;
use crate::macho;

#[test]
fn rejects_segment_virtual_range_overflow() {
    let mut image = valid_image();
    image.segments[COUNTERS].address = u64::MAX - PROFILE_PAGE_BYTES + 1;
    image.segments[COUNTERS].sections[FIRST_SECTION].address = image.segments[COUNTERS].address;
    assert_rejected(&image);
    assert!(macho::verify_profile_layout(&image).is_err());
}

#[test]
fn rejects_segment_file_range_overflow() {
    let mut bytes = encode(&valid_image());
    put_u64(&mut bytes, HEADER_BYTES + SEGMENT_FILE_OFFSET, u64::MAX);
    assert!(validate(&bytes).is_err());
    let mut image = valid_image();
    image.segments[COUNTERS].file_offset = u64::MAX;
    assert!(macho::verify_profile_layout(&image).is_err());
}

#[test]
fn rejects_section_virtual_range_and_page_rounding_overflow() {
    for size in [u64::MAX, u64::MAX - PROFILE_PAGE_BYTES + 2] {
        let mut image = valid_image();
        image.segments[COUNTERS].sections[FIRST_SECTION].size = size;
        assert_rejected(&image);
        assert!(macho::verify_profile_layout(&image).is_err());
    }
}

#[test]
fn rejects_section_file_range_overflow_in_supplied_image() {
    let mut image = valid_image();
    let counters = &mut image.segments[COUNTERS].sections[FIRST_SECTION];
    counters.file_offset = u64::MAX;
    assert!(macho::verify_profile_layout(&image).is_err());
}
