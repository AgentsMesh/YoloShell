use crate::constants::*;
use crate::layout_constants::*;
use crate::layout_fixture::*;
use crate::macho;

#[test]
fn parses_real_thin_macho_records_and_accepts_isolated_profile_pages() {
    let bytes = encode(&valid_image());
    let image = macho::inspect(&bytes).unwrap();
    assert_eq!(image.file_size, FILE_BYTES);
    assert_eq!(image.segments.len(), 3);
    let counters = &image.segments[COUNTERS];
    assert_eq!(counters.name, COUNTER_SEGMENT);
    assert_eq!(counters.address, BASE_ADDRESS + PROFILE_PAGE_BYTES);
    assert_eq!(counters.size, PROFILE_PAGE_BYTES);
    assert_eq!(counters.file_offset, PROFILE_PAGE_BYTES);
    assert_eq!(counters.file_size, PROFILE_PAGE_BYTES);
    assert_eq!(counters.max_protection, READ_WRITE);
    assert_eq!(counters.initial_protection, READ_WRITE);
    let section = &counters.sections[FIRST_SECTION];
    assert_eq!(section.name, COUNTER_SECTION);
    assert_eq!(section.segment, COUNTER_SEGMENT);
    assert_eq!(section.address, counters.address);
    assert_eq!(section.size, COUNTER_BYTES);
    assert_eq!(section.file_offset, counters.file_offset);
    assert_eq!(section.alignment, PAGE_ALIGNMENT_EXPONENT);
    assert_eq!(section.flags, 0);
    macho::verify_profile_layout(&image).unwrap();
}

#[test]
fn accepts_empty_bitmap_with_its_dedicated_page_and_valid_mapping() {
    let mut image = valid_image();
    image.segments[BITMAP].sections[FIRST_SECTION].size = 0;
    validate(&encode(&image)).unwrap();
}

#[test]
fn accepts_section_filling_the_dedicated_page_exactly() {
    let mut image = valid_image();
    image.segments[COUNTERS].sections[FIRST_SECTION].size = PROFILE_PAGE_BYTES;
    validate(&encode(&image)).unwrap();
}

#[test]
fn accepts_counter_payload_spanning_two_whole_owned_pages() {
    let mut image = valid_image();
    let counters = &mut image.segments[COUNTERS];
    counters.size = PROFILE_PAGE_BYTES * 2;
    counters.file_size = counters.size;
    counters.sections[FIRST_SECTION].size = PROFILE_PAGE_BYTES + COUNTER_BYTES;
    for segment in &mut image.segments[BITMAP..] {
        segment.address += PROFILE_PAGE_BYTES;
        segment.file_offset += PROFILE_PAGE_BYTES;
        for section in &mut segment.sections {
            section.address += PROFILE_PAGE_BYTES;
            section.file_offset += PROFILE_PAGE_BYTES;
        }
    }
    let mut bytes = encode(&image);
    bytes.resize((FILE_BYTES + PROFILE_PAGE_BYTES) as usize, 0);
    validate(&bytes).unwrap();
}

#[test]
fn accepts_unrelated_data_after_profile_data() {
    let mut image = valid_image();
    let data = &mut image.segments[DATA];
    let mut neighbor = section(
        DATA_SEGMENT,
        NEIGHBOR_SECTION,
        data.address + DATA_BYTES,
        DATA_BYTES,
        data.file_offset + DATA_BYTES,
    );
    neighbor.alignment = 0;
    data.sections.push(neighbor);
    validate(&encode(&image)).unwrap();
}
