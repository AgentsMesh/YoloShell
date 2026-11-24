use anyhow::{Context, Result, ensure};

use crate::constants::*;
use crate::model::EndRecord;
use crate::source::{ZipSource, fixed, u16_at, u32_at};

pub(crate) fn parse(source: &dyn ZipSource) -> Result<EndRecord> {
    let archive_length = source.length()?;
    ensure!(
        archive_length >= EOCD_BYTES as u64,
        "ZIP archive is shorter than EOCD"
    );
    ensure!(
        archive_length <= MAX_ARCHIVE_BYTES,
        "ZIP archive exceeds the size budget"
    );
    let eocd_offset = archive_length
        .checked_sub(EOCD_BYTES as u64)
        .context("ZIP EOCD offset underflow")?;
    let header = fixed::<EOCD_BYTES>(source, eocd_offset)?;
    ensure!(
        u32_at(&header, SIGNATURE_OFFSET) == EOCD_SIGNATURE,
        "ZIP EOCD must be the final fixed-size record"
    );
    ensure!(
        u16_at(&header, EOCD_COMMENT_LENGTH_OFFSET) == EMPTY_FIELD_LENGTH,
        "ZIP archive comments are unsupported"
    );
    ensure_single_disk(&header)?;
    let disk_entries = u16_at(&header, EOCD_DISK_ENTRIES_OFFSET);
    let total_entries = u16_at(&header, EOCD_TOTAL_ENTRIES_OFFSET);
    ensure!(
        disk_entries != ZIP64_U16_SENTINEL && total_entries != ZIP64_U16_SENTINEL,
        "Zip64 entry counts are unsupported"
    );
    ensure!(disk_entries == total_entries, "ZIP entry counts disagree");
    let entries = usize::from(total_entries);
    ensure!(
        entries >= MIN_ENTRY_COUNT,
        "ZIP archive must contain entries"
    );
    ensure!(entries <= MAX_ENTRY_COUNT, "ZIP entry count exceeds budget");
    let central_size = u64::from(u32_at(&header, EOCD_CENTRAL_SIZE_OFFSET));
    let central_offset = u64::from(u32_at(&header, EOCD_CENTRAL_OFFSET_OFFSET));
    ensure_non_zip64(central_size, central_offset)?;
    ensure!(
        central_size <= MAX_CENTRAL_BYTES,
        "ZIP central directory exceeds budget"
    );
    let minimum_size = (entries as u64)
        .checked_mul(CENTRAL_HEADER_BYTES as u64)
        .context("ZIP central directory minimum overflow")?;
    ensure!(
        central_size >= minimum_size,
        "ZIP central directory is too small for its entry count"
    );
    ensure!(
        central_offset.checked_add(central_size) == Some(eocd_offset),
        "ZIP central directory does not end at EOCD"
    );
    Ok(EndRecord {
        entries,
        central_offset,
        central_size,
    })
}

fn ensure_single_disk(header: &[u8]) -> Result<()> {
    ensure!(
        u16_at(header, EOCD_DISK_OFFSET) == PRIMARY_DISK
            && u16_at(header, EOCD_CENTRAL_DISK_OFFSET) == PRIMARY_DISK,
        "multi-disk ZIP archives are unsupported"
    );
    Ok(())
}

fn ensure_non_zip64(size: u64, offset: u64) -> Result<()> {
    let sentinel = u64::from(ZIP64_U32_SENTINEL);
    ensure!(
        size != sentinel && offset != sentinel,
        "Zip64 central directory fields are unsupported"
    );
    Ok(())
}
