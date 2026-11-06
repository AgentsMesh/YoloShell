use anyhow::{Context, Result};
use std::fs::File;
use std::os::unix::fs::FileExt;

use crate::constants::{SIGNATURE_BYTES, U16_SECOND_BYTE_OFFSET};

pub(crate) trait ZipSource {
    fn length(&self) -> Result<u64>;
    fn read_exact(&self, offset: u64, output: &mut [u8]) -> Result<()>;
}

impl ZipSource for File {
    fn length(&self) -> Result<u64> {
        self.metadata()
            .map(|value| value.len())
            .context("read pinned ZIP length")
    }

    fn read_exact(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        self.read_exact_at(output, offset)
            .context("read pinned ZIP bytes")
    }
}

pub(crate) struct ByteSource<'a>(pub &'a [u8]);

impl ZipSource for ByteSource<'_> {
    fn length(&self) -> Result<u64> {
        Ok(self.0.len() as u64)
    }

    fn read_exact(&self, offset: u64, output: &mut [u8]) -> Result<()> {
        let start = usize::try_from(offset)?;
        let end = start
            .checked_add(output.len())
            .context("ZIP range overflow")?;
        let source = self.0.get(start..end).context("truncated ZIP range")?;
        output.copy_from_slice(source);
        Ok(())
    }
}

pub(crate) fn fixed<const SIZE: usize>(source: &dyn ZipSource, offset: u64) -> Result<[u8; SIZE]> {
    let mut bytes = [0; SIZE];
    source.read_exact(offset, &mut bytes)?;
    Ok(bytes)
}

pub(crate) fn bytes(source: &dyn ZipSource, offset: u64, length: usize) -> Result<Vec<u8>> {
    let mut result = vec![0; length];
    source.read_exact(offset, &mut result)?;
    Ok(result)
}

pub(crate) fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + U16_SECOND_BYTE_OFFSET]])
}

pub(crate) fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + SIGNATURE_BYTES]
            .try_into()
            .expect("fixed ZIP field"),
    )
}
