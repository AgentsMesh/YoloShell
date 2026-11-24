use super::{
    Image, Section, Segment,
    constants::*,
    geometry,
    reader::{contains, end, overlaps},
};
use crate::constants::{
    BITMAP_SECTION, BITMAP_SEGMENT, COUNTER_SECTION, COUNTER_SEGMENT, DATA_SEGMENT,
    PROFILE_DATA_SECTION, PROFILE_PAGE_BYTES,
};

pub fn verify_profile_layout(image: &Image) -> Result<(), String> {
    geometry::validate(image)?;
    if !PROFILE_PAGE_BYTES.is_power_of_two() {
        return Err("invalid profile page-size policy".into());
    }
    dedicated(image, COUNTER_SEGMENT, COUNTER_SECTION)?;
    dedicated(image, BITMAP_SEGMENT, BITMAP_SECTION)?;
    let (segment, section) = unique_section(image, PROFILE_DATA_SECTION)?;
    if segment.name != DATA_SEGMENT
        || section.address % PROFILE_PAGE_BYTES != 0
        || section.flags & SECTION_TYPE_MASK != SECTION_REGULAR
    {
        return Err(format!(
            "profile data must be a page-aligned regular section in {DATA_SEGMENT}"
        ));
    }
    Ok(())
}

fn unique_section<'a>(image: &'a Image, name: &str) -> Result<(&'a Segment, &'a Section), String> {
    let mut matches = image.segments.iter().flat_map(|segment| {
        segment
            .sections
            .iter()
            .filter(move |section| section.name == name)
            .map(move |section| (segment, section))
    });
    let found = matches
        .next()
        .ok_or_else(|| format!("missing profile section {name}"))?;
    if matches.next().is_some() {
        return Err(format!("duplicate profile section {name}"));
    }
    Ok(found)
}

fn dedicated(image: &Image, segment_name: &str, section_name: &str) -> Result<(), String> {
    let (segment, section) = unique_section(image, section_name)?;
    if segment.name != segment_name || segment.sections.len() != 1 {
        return Err(format!(
            "{section_name} requires its own {segment_name} segment"
        ));
    }
    let writable = PROTECTION_READ | PROTECTION_WRITE;
    if segment.initial_protection & writable != writable
        || segment.max_protection & writable != writable
        || (segment.initial_protection | segment.max_protection) & PROTECTION_EXECUTE != 0
        || segment.flags & SEGMENT_READ_ONLY != 0
    {
        return Err(format!(
            "profile segment {segment_name} is not exclusively read/write"
        ));
    }
    if segment.address % PROFILE_PAGE_BYTES != 0
        || segment.size % PROFILE_PAGE_BYTES != 0
        || segment.file_offset % PROFILE_PAGE_BYTES != 0
        || section.address % PROFILE_PAGE_BYTES != 0
        || section.flags & SECTION_TYPE_MASK != SECTION_REGULAR
        || section.flags & !(SECTION_TYPE_MASK | PROFILE_ALLOWED_ATTRIBUTES) != 0
    {
        return Err(format!(
            "profile segment {segment_name} does not own whole regular pages"
        ));
    }
    let length = section
        .size
        .checked_add(PROFILE_PAGE_BYTES - 1)
        .ok_or("profile section page rounding overflow")?
        & !(PROFILE_PAGE_BYTES - 1);
    end(section.address, length)?;
    if !contains(segment.address, segment.size, section.address, length)? {
        return Err(format!(
            "rounded profile section {section_name} escapes its segment"
        ));
    }
    for other_segment in &image.segments {
        if !std::ptr::eq(segment, other_segment)
            && overlaps(
                section.address,
                length,
                other_segment.address,
                other_segment.size,
            )?
        {
            return Err(format!(
                "profile page overlaps segment {}",
                other_segment.name
            ));
        }
        for other in &other_segment.sections {
            if !std::ptr::eq(section, other)
                && overlaps(section.address, length, other.address, other.size)?
            {
                return Err(format!("profile page overlaps section {}", other.name));
            }
        }
    }
    Ok(())
}
