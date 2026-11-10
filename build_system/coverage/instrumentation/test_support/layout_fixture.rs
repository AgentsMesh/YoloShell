use crate::constants::*;
use crate::layout_constants::*;
use crate::macho::{self, Image, Section, Segment};

pub fn valid_image() -> Image {
    let segments = [
        (COUNTER_SEGMENT, COUNTER_SECTION, COUNTER_BYTES),
        (BITMAP_SEGMENT, BITMAP_SECTION, BITMAP_BYTES),
        (DATA_SEGMENT, PROFILE_DATA_SECTION, DATA_BYTES),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (name, section_name, section_size))| {
        let file_offset = (index as u64 + 1) * PROFILE_PAGE_BYTES;
        let address = BASE_ADDRESS + file_offset;
        Segment {
            name: name.into(),
            address,
            size: PROFILE_PAGE_BYTES,
            file_offset,
            file_size: PROFILE_PAGE_BYTES,
            max_protection: READ_WRITE,
            initial_protection: READ_WRITE,
            flags: 0,
            sections: vec![section(
                name,
                section_name,
                address,
                section_size,
                file_offset,
            )],
        }
    })
    .collect();
    Image {
        segments,
        file_size: FILE_BYTES,
    }
}

pub fn section(segment: &str, name: &str, address: u64, size: u64, file_offset: u64) -> Section {
    Section {
        name: name.into(),
        segment: segment.into(),
        address,
        size,
        file_offset,
        flags: 0,
        alignment: PAGE_ALIGNMENT_EXPONENT,
    }
}

pub fn encode(image: &Image) -> Vec<u8> {
    let command_bytes = image.segments.iter().map(command_size).sum::<usize>();
    let mut bytes = vec![0; FILE_BYTES as usize];
    put_u32(&mut bytes, MAGIC_OFFSET, MAGIC_64);
    put_u32(&mut bytes, CPU_OFFSET, CPU_ARM64);
    put_u32(&mut bytes, FILE_TYPE_OFFSET, EXECUTE_FILE);
    put_u32(
        &mut bytes,
        COMMAND_COUNT_OFFSET,
        image.segments.len() as u32,
    );
    put_u32(&mut bytes, COMMAND_BYTES_OFFSET, command_bytes as u32);
    let mut cursor = HEADER_BYTES;
    for segment in &image.segments {
        put_u32(&mut bytes, cursor, SEGMENT_64);
        put_u32(
            &mut bytes,
            cursor + COMMAND_SIZE_OFFSET,
            command_size(segment) as u32,
        );
        put_name(&mut bytes, cursor + SEGMENT_NAME_OFFSET, &segment.name);
        for (offset, value) in [
            (SEGMENT_ADDRESS_OFFSET, segment.address),
            (SEGMENT_SIZE_OFFSET, segment.size),
            (SEGMENT_FILE_OFFSET, segment.file_offset),
            (SEGMENT_FILE_SIZE_OFFSET, segment.file_size),
        ] {
            put_u64(&mut bytes, cursor + offset, value);
        }
        for (offset, value) in [
            (MAX_PROTECTION_OFFSET, segment.max_protection),
            (INITIAL_PROTECTION_OFFSET, segment.initial_protection),
            (SECTION_COUNT_OFFSET, segment.sections.len() as u32),
            (SEGMENT_FLAGS_OFFSET, segment.flags),
        ] {
            put_u32(&mut bytes, cursor + offset, value);
        }
        let mut section_cursor = cursor + SEGMENT_BYTES;
        for section in &segment.sections {
            put_name(&mut bytes, section_cursor, &section.name);
            put_name(
                &mut bytes,
                section_cursor + SECTION_SEGMENT_OFFSET,
                &section.segment,
            );
            put_u64(
                &mut bytes,
                section_cursor + SECTION_ADDRESS_OFFSET,
                section.address,
            );
            put_u64(
                &mut bytes,
                section_cursor + SECTION_SIZE_OFFSET,
                section.size,
            );
            put_u32(
                &mut bytes,
                section_cursor + SECTION_FILE_OFFSET,
                section.file_offset as u32,
            );
            put_u32(
                &mut bytes,
                section_cursor + SECTION_ALIGNMENT_OFFSET,
                section.alignment,
            );
            put_u32(
                &mut bytes,
                section_cursor + SECTION_FLAGS_OFFSET,
                section.flags,
            );
            section_cursor += SECTION_BYTES;
        }
        cursor += command_size(segment);
    }
    bytes
}

fn command_size(segment: &Segment) -> usize {
    SEGMENT_BYTES + segment.sections.len() * SECTION_BYTES
}

fn put_name(bytes: &mut [u8], offset: usize, value: &str) {
    assert!(value.len() <= NAME_BYTES);
    bytes[offset..offset + value.len()].copy_from_slice(value.as_bytes());
}

pub fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + size_of::<u32>()].copy_from_slice(&value.to_le_bytes());
}

pub fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + size_of::<u64>()].copy_from_slice(&value.to_le_bytes());
}

pub fn validate(bytes: &[u8]) -> Result<(), String> {
    macho::inspect(bytes).and_then(|image| macho::verify_profile_layout(&image))
}

pub fn assert_rejected(image: &Image) {
    assert!(
        validate(&encode(image)).is_err(),
        "invalid layout was accepted"
    );
}
