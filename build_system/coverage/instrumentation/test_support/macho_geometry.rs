use super::{
    Image, Section, Segment,
    constants::*,
    reader::{contains, end, overlaps},
};
use std::collections::BTreeSet;

pub(super) fn validate(image: &Image) -> Result<(), String> {
    if image.segments.is_empty() || image.segments.len() > MAX_SEGMENTS {
        return Err("invalid Mach-O segment count".into());
    }
    let mut names = BTreeSet::new();
    let mut count = 0usize;
    for segment in &image.segments {
        if !names.insert(&segment.name) {
            return Err(format!("duplicate Mach-O segment {}", segment.name));
        }
        count = count
            .checked_add(segment.sections.len())
            .ok_or("Mach-O section count overflow")?;
        if count > MAX_SECTIONS {
            return Err("excessive Mach-O section count".into());
        }
        validate_segment(segment, image.file_size)?;
    }
    for (index, segment) in image.segments.iter().enumerate() {
        for other in &image.segments[index + 1..] {
            if overlaps(segment.address, segment.size, other.address, other.size)?
                || overlaps(
                    segment.file_offset,
                    segment.file_size,
                    other.file_offset,
                    other.file_size,
                )?
            {
                return Err(format!(
                    "overlapping Mach-O segments {} and {}",
                    segment.name, other.name
                ));
            }
        }
    }
    Ok(())
}

fn validate_segment(segment: &Segment, file_size: u64) -> Result<(), String> {
    valid_name(&segment.name)?;
    end(segment.address, segment.size)?;
    if end(segment.file_offset, segment.file_size)? > file_size || segment.file_size > segment.size
    {
        return Err(format!("invalid Mach-O file mapping for {}", segment.name));
    }
    if (segment.initial_protection | segment.max_protection) & !KNOWN_PROTECTIONS != 0
        || segment.initial_protection & !segment.max_protection != 0
        || segment.flags & (!SEGMENT_FLAGS_MASK | SEGMENT_HIGH_VM) != 0
    {
        return Err(format!(
            "unsupported Mach-O mapping flags for {}",
            segment.name
        ));
    }
    let mut names = BTreeSet::new();
    let mut ranges = Vec::with_capacity(segment.sections.len());
    for section in &segment.sections {
        if !names.insert(&section.name) {
            return Err(format!("duplicate Mach-O section {}", section.name));
        }
        validate_section(segment, section)?;
        if section.size != 0 {
            ranges.push((section.address, end(section.address, section.size)?));
        }
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err(format!("overlapping Mach-O sections in {}", segment.name));
    }
    Ok(())
}

fn validate_section(segment: &Segment, section: &Section) -> Result<(), String> {
    valid_name(&section.name)?;
    if section.segment != segment.name {
        return Err(format!(
            "Mach-O section {} belongs to a different segment",
            section.name
        ));
    }
    if section.alignment > MAX_ALIGNMENT_EXPONENT
        || section.address % (1u64 << section.alignment) != 0
        || section.flags & SECTION_TYPE_MASK > LAST_KNOWN_SECTION_TYPE
    {
        return Err(format!(
            "invalid Mach-O section flags or alignment for {}",
            section.name
        ));
    }
    if !contains(segment.address, segment.size, section.address, section.size)? {
        return Err(format!(
            "Mach-O section {} escapes its segment",
            section.name
        ));
    }
    // Empty sections have no file payload; linkers may encode their offset as zero.
    if section.size == 0 && section.file_offset == 0 {
        return Ok(());
    }
    if is_zero_fill(section) {
        if section.file_offset != 0 {
            return Err("zero-fill Mach-O section has a file offset".into());
        }
    } else if !contains(
        segment.file_offset,
        segment.file_size,
        section.file_offset,
        section.size,
    )? || section.file_offset - segment.file_offset != section.address - segment.address
    {
        return Err(format!(
            "invalid Mach-O section file mapping for {}",
            section.name
        ));
    }
    Ok(())
}

pub(super) fn is_zero_fill(section: &Section) -> bool {
    matches!(
        section.flags & SECTION_TYPE_MASK,
        SECTION_ZERO_FILL | SECTION_GB_ZERO_FILL | SECTION_THREAD_ZERO_FILL
    )
}

fn valid_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > NAME_BYTES
        || !name.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err("invalid Mach-O segment or section name".into());
    }
    Ok(())
}
