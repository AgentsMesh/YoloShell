use super::{Section, Segment, constants::*, reader::Reader};

pub(super) fn segment(reader: &Reader<'_>, total: &mut usize) -> Result<Segment, String> {
    reader.slice(0, SEGMENT_BYTES)?;
    let count = usize::try_from(reader.word(SEGMENT_SECTION_COUNT_OFFSET)?)
        .map_err(|_| "Mach-O section count exceeds platform limits")?;
    *total = total
        .checked_add(count)
        .ok_or("Mach-O section count overflow")?;
    if *total > MAX_SECTIONS {
        return Err("excessive Mach-O section count".into());
    }
    let section_bytes = count
        .checked_mul(SECTION_BYTES)
        .ok_or("Mach-O section bytes overflow")?;
    let length = SEGMENT_BYTES
        .checked_add(section_bytes)
        .ok_or("Mach-O segment size overflow")?;
    if reader.0.len() != length {
        return Err("Mach-O segment command size does not match its section count".into());
    }
    let mut segment = Segment {
        name: reader.name(SEGMENT_NAME_OFFSET)?,
        address: reader.address(SEGMENT_ADDRESS_OFFSET)?,
        size: reader.address(SEGMENT_SIZE_OFFSET)?,
        file_offset: reader.address(SEGMENT_FILE_OFFSET)?,
        file_size: reader.address(SEGMENT_FILE_SIZE_OFFSET)?,
        max_protection: reader.word(SEGMENT_MAX_PROTECTION_OFFSET)?,
        initial_protection: reader.word(SEGMENT_INITIAL_PROTECTION_OFFSET)?,
        flags: reader.word(SEGMENT_FLAGS_OFFSET)?,
        sections: Vec::with_capacity(count),
    };
    for bytes in reader
        .slice(SEGMENT_BYTES, section_bytes)?
        .chunks_exact(SECTION_BYTES)
    {
        let reader = Reader(bytes);
        segment.sections.push(Section {
            name: reader.name(SECTION_NAME_OFFSET)?,
            segment: reader.name(SECTION_SEGMENT_OFFSET)?,
            address: reader.address(SECTION_ADDRESS_OFFSET)?,
            size: reader.address(SECTION_SIZE_OFFSET)?,
            file_offset: u64::from(reader.word(SECTION_FILE_OFFSET)?),
            alignment: reader.word(SECTION_ALIGNMENT_OFFSET)?,
            flags: reader.word(SECTION_FLAGS_OFFSET)?,
        });
    }
    Ok(segment)
}
