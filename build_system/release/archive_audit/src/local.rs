use anyhow::{Context, Result, ensure};

use crate::constants::*;
use crate::model::{CentralEntry, EntrySpan};
use crate::payload;
use crate::source::{ZipSource, bytes, fixed, u16_at, u32_at};

pub(crate) fn validate(
    source: &dyn ZipSource,
    entries: &[CentralEntry],
    central_offset: u64,
) -> Result<Vec<String>> {
    let mut spans = Vec::with_capacity(entries.len());
    let mut digests = Vec::with_capacity(entries.len());
    for entry in entries {
        let (span, digest) = validate_entry(source, entry, central_offset)?;
        spans.push(span);
        digests.push(digest);
    }
    spans.sort_unstable_by_key(|span| span.start);
    ensure!(
        spans
            .first()
            .is_some_and(|span| span.start == ARCHIVE_START_OFFSET),
        "ZIP local entries have a preamble or missing first entry"
    );
    for pair in spans.windows(ADJACENT_SPAN_WINDOW) {
        ensure!(
            pair[0].end == pair[1].start,
            "ZIP local entries overlap or contain an unknown gap"
        );
    }
    ensure!(
        spans.last().is_some_and(|span| span.end == central_offset),
        "ZIP local entries do not end at the central directory"
    );
    Ok(digests)
}

fn validate_entry(
    source: &dyn ZipSource,
    entry: &CentralEntry,
    central_offset: u64,
) -> Result<(EntrySpan, String)> {
    ensure!(
        entry.local_offset < central_offset,
        "ZIP local header starts in the central directory"
    );
    let header = fixed::<LOCAL_HEADER_BYTES>(source, entry.local_offset)?;
    ensure!(
        u32_at(&header, SIGNATURE_OFFSET) == LOCAL_SIGNATURE,
        "ZIP local header signature mismatch"
    );
    ensure_bound_fields(&header, entry)?;
    let name_length = usize::from(u16_at(&header, LOCAL_NAME_LENGTH_OFFSET));
    ensure!(
        name_length == entry.name.len(),
        "ZIP local and central name lengths differ"
    );
    ensure!(
        u16_at(&header, LOCAL_EXTRA_LENGTH_OFFSET) == EMPTY_FIELD_LENGTH,
        "ZIP local extra fields are unsupported"
    );
    let name_offset = entry
        .local_offset
        .checked_add(LOCAL_HEADER_BYTES as u64)
        .context("ZIP local name offset overflow")?;
    ensure!(
        bytes(source, name_offset, name_length)? == entry.name,
        "ZIP local and central raw names differ"
    );
    let data_start = name_offset
        .checked_add(name_length as u64)
        .context("ZIP local data offset overflow")?;
    let data_end = data_start
        .checked_add(entry.compressed_size)
        .context("ZIP local data span overflow")?;
    ensure!(
        data_end <= central_offset,
        "ZIP local entry reaches the central directory"
    );
    let digest = payload::validate(source, entry, data_start)?;
    Ok((
        EntrySpan {
            start: entry.local_offset,
            end: data_end,
        },
        digest,
    ))
}

fn ensure_bound_fields(header: &[u8], entry: &CentralEntry) -> Result<()> {
    ensure!(
        u16_at(header, LOCAL_NEEDED_OFFSET) == entry.version_needed,
        "ZIP local and central extraction versions differ"
    );
    ensure!(
        u16_at(header, LOCAL_FLAGS_OFFSET) == entry.flags,
        "ZIP local and central flags differ"
    );
    ensure!(
        u16_at(header, LOCAL_METHOD_OFFSET) == entry.method,
        "ZIP local and central methods differ"
    );
    ensure!(
        u16_at(header, LOCAL_TIME_OFFSET) == entry.modified_time
            && u16_at(header, LOCAL_DATE_OFFSET) == entry.modified_date,
        "ZIP local and central timestamps differ"
    );
    ensure!(
        u32_at(header, LOCAL_CRC_OFFSET) == entry.crc32,
        "ZIP local and central CRC values differ"
    );
    ensure!(
        u64::from(u32_at(header, LOCAL_COMPRESSED_OFFSET)) == entry.compressed_size
            && u64::from(u32_at(header, LOCAL_UNCOMPRESSED_OFFSET)) == entry.uncompressed_size,
        "ZIP local and central sizes differ"
    );
    Ok(())
}
