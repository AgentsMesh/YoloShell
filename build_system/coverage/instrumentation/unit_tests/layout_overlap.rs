use crate::constants::*;
use crate::layout_constants::*;
use crate::layout_fixture::*;

#[test]
fn rejects_unknown_section_sharing_a_profile_page() {
    let mut image = valid_image();
    let counters = &mut image.segments[COUNTERS];
    let mut neighbor = section(
        COUNTER_SEGMENT,
        NEIGHBOR_SECTION,
        counters.address + COUNTER_BYTES,
        DATA_BYTES,
        counters.file_offset + COUNTER_BYTES,
    );
    neighbor.alignment = 0;
    counters.sections.push(neighbor);
    assert_rejected(&image);
}

#[test]
fn dedicated_segment_cannot_contain_a_neighbor_on_another_page() {
    let mut image = valid_image();
    let mut counters = image.segments.remove(COUNTERS);
    counters.size = PROFILE_PAGE_BYTES * 2;
    counters.file_size = counters.size;
    counters.sections.push(section(
        COUNTER_SEGMENT,
        NEIGHBOR_SECTION,
        counters.address + PROFILE_PAGE_BYTES,
        DATA_BYTES,
        counters.file_offset + PROFILE_PAGE_BYTES,
    ));
    for segment in &mut image.segments {
        segment.address += PROFILE_PAGE_BYTES;
        segment.file_offset += PROFILE_PAGE_BYTES;
        for section in &mut segment.sections {
            section.address += PROFILE_PAGE_BYTES;
            section.file_offset += PROFILE_PAGE_BYTES;
        }
    }
    image.segments.insert(COUNTERS, counters);
    let mut bytes = encode(&image);
    bytes.resize((FILE_BYTES + PROFILE_PAGE_BYTES) as usize, 0);
    assert!(validate(&bytes).is_err());
}

#[test]
fn rounded_profile_mapping_must_fit_its_own_segment() {
    let mut image = valid_image();
    let counters = &mut image.segments[COUNTERS];
    counters.size = COUNTER_BYTES;
    counters.file_size = COUNTER_BYTES;
    assert_rejected(&image);
}

#[test]
fn rejects_another_segment_overlapping_the_profile_page() {
    let mut image = valid_image();
    let mut neighbor = valid_image().segments.remove(DATA);
    neighbor.name = NEIGHBOR_SEGMENT.into();
    neighbor.address = image.segments[COUNTERS].address + COUNTER_BYTES;
    neighbor.size = DATA_BYTES;
    neighbor.file_size = DATA_BYTES;
    neighbor.sections.clear();
    image.segments.push(neighbor);
    assert_rejected(&image);
}

#[test]
fn rejects_distinct_virtual_segments_aliasing_the_same_file_page() {
    let mut image = valid_image();
    let offset = image.segments[COUNTERS].file_offset;
    image.segments[BITMAP].file_offset = offset;
    image.segments[BITMAP].sections[FIRST_SECTION].file_offset = offset;
    assert_rejected(&image);
}

#[test]
fn rejects_profile_section_outside_its_own_segment() {
    let mut image = valid_image();
    let bitmap_address = image.segments[BITMAP].address;
    let bitmap_offset = image.segments[BITMAP].file_offset;
    let counters = &mut image.segments[COUNTERS].sections[FIRST_SECTION];
    counters.address = bitmap_address;
    counters.file_offset = bitmap_offset;
    assert_rejected(&image);
}
