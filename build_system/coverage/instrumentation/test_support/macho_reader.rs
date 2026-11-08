use super::constants::*;

pub(super) struct Reader<'a>(pub(super) &'a [u8]);

impl<'a> Reader<'a> {
    pub(super) fn slice(&self, offset: usize, size: usize) -> Result<&'a [u8], String> {
        let end = offset
            .checked_add(size)
            .ok_or("Mach-O byte range overflow")?;
        self.0
            .get(offset..end)
            .ok_or_else(|| "truncated Mach-O input".into())
    }

    pub(super) fn word(&self, offset: usize) -> Result<u32, String> {
        let bytes = self.slice(offset, WORD_BYTES)?;
        Ok(u32::from_le_bytes(
            bytes.try_into().map_err(|_| "invalid Mach-O word")?,
        ))
    }

    pub(super) fn address(&self, offset: usize) -> Result<u64, String> {
        let bytes = self.slice(offset, ADDRESS_BYTES)?;
        Ok(u64::from_le_bytes(
            bytes.try_into().map_err(|_| "invalid Mach-O address")?,
        ))
    }

    pub(super) fn name(&self, offset: usize) -> Result<String, String> {
        let bytes = self.slice(offset, NAME_BYTES)?;
        let length = bytes
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(NAME_BYTES);
        if length == 0
            || bytes[length..].iter().any(|byte| *byte != 0)
            || bytes[..length].iter().any(|byte| !byte.is_ascii_graphic())
        {
            return Err("invalid Mach-O segment or section name".into());
        }
        String::from_utf8(bytes[..length].to_vec()).map_err(|error| error.to_string())
    }
}

pub(super) fn end(start: u64, size: u64) -> Result<u64, String> {
    start
        .checked_add(size)
        .ok_or_else(|| "Mach-O address range overflow".into())
}

pub(super) fn contains(start: u64, size: u64, inner: u64, length: u64) -> Result<bool, String> {
    Ok(inner >= start && end(inner, length)? <= end(start, size)?)
}

pub(super) fn overlaps(start: u64, size: u64, other: u64, length: u64) -> Result<bool, String> {
    let stop = end(start, size)?;
    let other_stop = end(other, length)?;
    Ok(size != 0 && length != 0 && start < other_stop && other < stop)
}
