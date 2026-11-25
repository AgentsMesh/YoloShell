use crate::constants::*;
use crate::layout_constants::*;
use crate::layout_fixture::*;

#[test]
fn rejects_legacy_profile_sections_in_the_data_segment() {
    let mut image = valid_image();
    let mut legacy_sections = Vec::new();
    for segment in image.segments.drain(..) {
        for mut section in segment.sections {
            section.segment = DATA_SEGMENT.into();
            legacy_sections.push(section);
        }
    }
    let mut legacy = valid_image().segments.remove(DATA);
    legacy.address = BASE_ADDRESS + PROFILE_PAGE_BYTES;
    legacy.file_offset = PROFILE_PAGE_BYTES;
    legacy.size = PROFILE_PAGE_BYTES * 3;
    legacy.file_size = legacy.size;
    legacy.sections = legacy_sections;
    image.segments.push(legacy);
    assert_rejected(&image);
}

#[test]
fn rejects_missing_required_segment_or_section() {
    for index in [COUNTERS, BITMAP, DATA] {
        let mut image = valid_image();
        image.segments.remove(index);
        assert_rejected(&image);
        let mut image = valid_image();
        image.segments[index].sections.clear();
        assert_rejected(&image);
    }
}

#[test]
fn rejects_duplicate_segments_and_profile_sections() {
    let mut image = valid_image();
    image.segments.push(valid_image().segments.remove(COUNTERS));
    assert_rejected(&image);
    for index in [COUNTERS, BITMAP, DATA] {
        let mut image = valid_image();
        let duplicate = valid_image()
            .segments
            .remove(index)
            .sections
            .remove(FIRST_SECTION);
        image.segments[index].sections.push(duplicate);
        assert_rejected(&image);
    }
}

#[test]
fn requires_read_write_without_execute_in_both_protection_fields() {
    for index in [COUNTERS, BITMAP] {
        for protection in [0, READ, WRITE, READ_WRITE | EXECUTE] {
            let mut image = valid_image();
            image.segments[index].max_protection = protection;
            assert_rejected(&image);
            let mut image = valid_image();
            image.segments[index].initial_protection = protection;
            assert_rejected(&image);
        }
    }
}

#[test]
fn rejects_read_only_segment_and_non_regular_profile_section_flags() {
    for index in [COUNTERS, BITMAP] {
        let mut image = valid_image();
        image.segments[index].flags = READ_ONLY_SEGMENT;
        assert_rejected(&image);
        for flags in [ZERO_FILL_SECTION, INSTRUCTIONS_SECTION] {
            let mut image = valid_image();
            image.segments[index].sections[FIRST_SECTION].flags = flags;
            assert_rejected(&image);
        }
    }
}

#[test]
fn rejects_section_with_wrong_owner_or_file_mapping() {
    for index in [COUNTERS, BITMAP, DATA] {
        let mut image = valid_image();
        image.segments[index].sections[FIRST_SECTION].segment = NEIGHBOR_SEGMENT.into();
        assert_rejected(&image);
        let mut image = valid_image();
        image.segments[index].sections[FIRST_SECTION].file_offset += 1;
        assert_rejected(&image);
    }
}

#[test]
fn rejects_profile_addresses_that_are_not_page_aligned() {
    for index in [COUNTERS, BITMAP, DATA] {
        let mut image = valid_image();
        let section = &mut image.segments[index].sections[FIRST_SECTION];
        section.address += 1;
        section.file_offset += 1;
        section.alignment = 0;
        assert_rejected(&image);
    }
}

#[test]
fn rejects_profile_data_moved_out_of_data_segment() {
    let mut image = valid_image();
    image.segments[DATA].name = NEIGHBOR_SEGMENT.into();
    image.segments[DATA].sections[FIRST_SECTION].segment = NEIGHBOR_SEGMENT.into();
    assert_rejected(&image);
}

#[test]
fn empty_bitmap_still_requires_a_valid_address_and_mapping() {
    for invalid_address in [0, BASE_ADDRESS + PROFILE_PAGE_BYTES] {
        let mut image = valid_image();
        let bitmap = &mut image.segments[BITMAP].sections[FIRST_SECTION];
        bitmap.size = 0;
        bitmap.address = invalid_address;
        assert_rejected(&image);
    }
}
