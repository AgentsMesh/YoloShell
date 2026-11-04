use anyhow::{Context, Result, ensure};
use std::collections::HashSet;

use crate::constants::*;
use crate::entry_path;
use crate::entry_type::{self, TypeFields};
use crate::model::{CentralEntry, EndRecord};
use crate::source::{ZipSource, bytes, fixed, u16_at, u32_at};

pub(crate) fn parse(source: &dyn ZipSource, end: &EndRecord) -> Result<Vec<CentralEntry>> {
    let central_end = end
        .central_offset
        .checked_add(end.central_size)
        .context("ZIP central directory overflow")?;
    let mut cursor = end.central_offset;
    let mut raw_names = HashSet::new();
    let mut aliases = HashSet::new();
    let mut total_uncompressed = 0_u64;
    let mut entries = Vec::with_capacity(end.entries);
    for _ in 0..end.entries {
        ensure_header_fits(cursor, central_end)?;
        let header = fixed::<CENTRAL_HEADER_BYTES>(source, cursor)?;
        ensure!(
            u32_at(&header, SIGNATURE_OFFSET) == CENTRAL_SIGNATURE,
            "ZIP central header signature mismatch"
        );
        validate_fixed_fields(&header)?;
        let name_length = usize::from(u16_at(&header, CENTRAL_NAME_LENGTH_OFFSET));
        let extra_length = usize::from(u16_at(&header, CENTRAL_EXTRA_LENGTH_OFFSET));
        let comment_length = usize::from(u16_at(&header, CENTRAL_COMMENT_LENGTH_OFFSET));
        ensure!(
            extra_length == usize::from(EMPTY_FIELD_LENGTH),
            "ZIP extra fields are unsupported"
        );
        ensure!(
            comment_length == usize::from(EMPTY_FIELD_LENGTH),
            "ZIP entry comments are unsupported"
        );
        let next = next_header(cursor, name_length, extra_length, comment_length)?;
        ensure!(next <= central_end, "ZIP central entry exceeds directory");
        let name_offset = cursor
            .checked_add(CENTRAL_HEADER_BYTES as u64)
            .context("ZIP central name offset overflow")?;
        let name = bytes(source, name_offset, name_length)?;
        let validated_path = entry_path::validate(&name)?;
        ensure!(
            raw_names.insert(name.clone()),
            "duplicate raw ZIP entry name"
        );
        ensure!(
            aliases.insert(validated_path.alias),
            "duplicate or case-folded ZIP path alias"
        );
        let entry = parse_entry(
            &header,
            name,
            validated_path.path,
            validated_path.has_directory_suffix,
        )?;
        total_uncompressed = total_uncompressed
            .checked_add(entry.uncompressed_size)
            .context("ZIP total uncompressed size overflow")?;
        ensure!(
            total_uncompressed <= MAX_TOTAL_UNCOMPRESSED_BYTES,
            "ZIP total uncompressed size exceeds budget"
        );
        entries.push(entry);
        cursor = next;
    }
    ensure!(
        cursor == central_end,
        "ZIP central directory has trailing or unparsed bytes"
    );
    Ok(entries)
}

fn parse_entry(
    header: &[u8],
    name: Vec<u8>,
    path: String,
    has_directory_suffix: bool,
) -> Result<CentralEntry> {
    let compressed_size = checked_u32_field(header, CENTRAL_COMPRESSED_OFFSET, "compressed size")?;
    let uncompressed_size =
        checked_u32_field(header, CENTRAL_UNCOMPRESSED_OFFSET, "uncompressed size")?;
    ensure_sizes(compressed_size, uncompressed_size)?;
    let method = u16_at(header, CENTRAL_METHOD_OFFSET);
    ensure!(
        method != STORED_METHOD || compressed_size == uncompressed_size,
        "stored ZIP entry size mismatch"
    );
    let crc32 = u32_at(header, CENTRAL_CRC_OFFSET);
    let validated_type = entry_type::validate(TypeFields {
        external_attributes: u32_at(header, CENTRAL_EXTERNAL_OFFSET),
        has_directory_suffix,
        method,
        crc32,
        compressed_size,
        uncompressed_size,
    })?;
    Ok(CentralEntry {
        version_needed: u16_at(header, CENTRAL_NEEDED_OFFSET),
        flags: u16_at(header, CENTRAL_FLAGS_OFFSET),
        method,
        modified_time: u16_at(header, CENTRAL_TIME_OFFSET),
        modified_date: u16_at(header, CENTRAL_DATE_OFFSET),
        crc32,
        compressed_size,
        uncompressed_size,
        name,
        path,
        mode: validated_type.mode,
        is_directory: validated_type.is_directory,
        local_offset: checked_u32_field(header, CENTRAL_LOCAL_OFFSET, "local offset")?,
    })
}

fn validate_fixed_fields(header: &[u8]) -> Result<()> {
    let made_by = u16_at(header, CENTRAL_MADE_BY_OFFSET);
    ensure!(
        (made_by >> u8::BITS) as u8 == UNIX_ORIGIN,
        "ZIP entry was not created with Unix attributes"
    );
    let needed = u16_at(header, CENTRAL_NEEDED_OFFSET);
    ensure!(
        (MIN_VERSION_NEEDED..=MAX_VERSION_NEEDED).contains(&needed),
        "unsupported ZIP extraction version"
    );
    ensure!(
        u16_at(header, CENTRAL_FLAGS_OFFSET) == SUPPORTED_FLAGS,
        "unsupported or encrypted ZIP flags"
    );
    // APPNOTE 4.4.14.1 defines bit 0 as an informational text hint. Payloads
    // still undergo the same raw-byte CRC/size/hash checks; other bits fail closed.
    ensure!(
        u16_at(header, CENTRAL_INTERNAL_OFFSET) & !TEXT_INTERNAL_ATTRIBUTE == EMPTY_FIELD_LENGTH,
        "ZIP internal attributes are unsupported"
    );
    ensure!(
        matches!(
            u16_at(header, CENTRAL_METHOD_OFFSET),
            STORED_METHOD | DEFLATE_METHOD
        ),
        "unsupported ZIP compression method"
    );
    let disk = u16_at(header, CENTRAL_DISK_OFFSET);
    ensure!(disk == PRIMARY_DISK, "multi-disk ZIP entry is unsupported");
    Ok(())
}

fn checked_u32_field(header: &[u8], offset: usize, label: &str) -> Result<u64> {
    let value = u32_at(header, offset);
    ensure!(value != ZIP64_U32_SENTINEL, "Zip64 {label} is unsupported");
    Ok(u64::from(value))
}

fn ensure_sizes(compressed: u64, uncompressed: u64) -> Result<()> {
    ensure!(
        compressed <= MAX_ENTRY_COMPRESSED_BYTES,
        "compressed ZIP entry exceeds budget"
    );
    ensure!(
        uncompressed <= MAX_ENTRY_UNCOMPRESSED_BYTES,
        "uncompressed ZIP entry exceeds budget"
    );
    let ratio_budget = compressed
        .checked_mul(MAX_COMPRESSION_RATIO)
        .and_then(|value| value.checked_add(COMPRESSION_RATIO_SLACK_BYTES))
        .context("ZIP compression ratio budget overflow")?;
    ensure!(
        uncompressed <= ratio_budget,
        "ZIP compression ratio exceeds budget"
    );
    Ok(())
}

fn ensure_header_fits(cursor: u64, end: u64) -> Result<()> {
    ensure!(
        cursor
            .checked_add(CENTRAL_HEADER_BYTES as u64)
            .is_some_and(|value| value <= end),
        "truncated ZIP central header"
    );
    Ok(())
}

fn next_header(cursor: u64, name: usize, extra: usize, comment: usize) -> Result<u64> {
    let variable = name
        .checked_add(extra)
        .and_then(|value| value.checked_add(comment))
        .context("ZIP central variable length overflow")?;
    let variable = u64::try_from(variable).context("ZIP central length conversion")?;
    cursor
        .checked_add(CENTRAL_HEADER_BYTES as u64)
        .and_then(|value| value.checked_add(variable))
        .context("ZIP central entry length overflow")
}
