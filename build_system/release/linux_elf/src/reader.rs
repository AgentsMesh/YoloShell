use yolo_release_contracts::ELF64_COMPLETE_HEADER_BYTES;
use anyhow::{Result, ensure};

const PROGRAM_HEADER_TABLE_OFFSET: usize = 32;
const PROGRAM_HEADER_ENTRY_SIZE_OFFSET: usize = 54;
const PROGRAM_HEADER_COUNT_OFFSET: usize = 56;
const PROGRAM_HEADER_BYTES: usize = 56;
const PROGRAM_TYPE_OFFSET: usize = 0;
const PROGRAM_FILE_OFFSET: usize = 8;
const PROGRAM_VIRTUAL_ADDRESS_OFFSET: usize = 16;
const PROGRAM_FILE_SIZE_OFFSET: usize = 32;
const PROGRAM_LOAD_TYPE: u32 = 1;
const PROGRAM_DYNAMIC_TYPE: u32 = 2;
const MAX_PROGRAM_HEADER_COUNT: usize = 4_096;
const ELF_BOUNDS_ERROR: &str = "Linux runtime ELF structure is out of bounds";
const ELF_PROGRAM_HEADER_ERROR: &str = "Linux runtime ELF program headers are invalid";
const ELF_VIRTUAL_ADDRESS_ERROR: &str = "Linux runtime ELF virtual address is not file-backed";

#[derive(Clone, Copy)]
pub(crate) struct Segment {
    kind: u32,
    file_offset: u64,
    virtual_address: u64,
    file_size: u64,
}

impl Segment {
    pub(crate) fn is_dynamic(self) -> bool {
        self.kind == PROGRAM_DYNAMIC_TYPE
    }
}

pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Result<Self> {
        ensure!(bytes.len() >= ELF64_COMPLETE_HEADER_BYTES, ELF_BOUNDS_ERROR);
        Ok(Self { bytes })
    }

    pub(crate) fn segments(&self) -> Result<Vec<Segment>> {
        let table_offset = usize::try_from(Self::u64(self.bytes, PROGRAM_HEADER_TABLE_OFFSET)?)?;
        let entry_size = usize::from(Self::u16(self.bytes, PROGRAM_HEADER_ENTRY_SIZE_OFFSET)?);
        let count = usize::from(Self::u16(self.bytes, PROGRAM_HEADER_COUNT_OFFSET)?);
        if count == 0 {
            return Ok(Vec::new());
        }
        ensure!(
            entry_size == PROGRAM_HEADER_BYTES && count <= MAX_PROGRAM_HEADER_COUNT,
            ELF_PROGRAM_HEADER_ERROR
        );
        let table_size = entry_size
            .checked_mul(count)
            .ok_or_else(|| anyhow::anyhow!(ELF_BOUNDS_ERROR))?;
        let table = Self::range(self.bytes, table_offset, table_size)?;
        (0..count)
            .map(|index| {
                let entry = Self::range(table, index * entry_size, entry_size)?;
                let segment = Segment {
                    kind: Self::u32(entry, PROGRAM_TYPE_OFFSET)?,
                    file_offset: Self::u64(entry, PROGRAM_FILE_OFFSET)?,
                    virtual_address: Self::u64(entry, PROGRAM_VIRTUAL_ADDRESS_OFFSET)?,
                    file_size: Self::u64(entry, PROGRAM_FILE_SIZE_OFFSET)?,
                };
                let _ = self.segment_bytes(segment)?;
                Ok(segment)
            })
            .collect()
    }

    pub(crate) fn segment_bytes(&self, segment: Segment) -> Result<&'a [u8]> {
        Self::range(
            self.bytes,
            usize::try_from(segment.file_offset)?,
            usize::try_from(segment.file_size)?,
        )
    }

    pub(crate) fn mapped(&self, segments: &[Segment], address: u64) -> Result<&'a [u8]> {
        for segment in segments
            .iter()
            .filter(|segment| segment.kind == PROGRAM_LOAD_TYPE)
        {
            let Some(delta) = address.checked_sub(segment.virtual_address) else {
                continue;
            };
            if delta >= segment.file_size {
                continue;
            }
            let offset = segment
                .file_offset
                .checked_add(delta)
                .ok_or_else(|| anyhow::anyhow!(ELF_BOUNDS_ERROR))?;
            return Self::range(
                self.bytes,
                usize::try_from(offset)?,
                usize::try_from(segment.file_size - delta)?,
            );
        }
        anyhow::bail!(ELF_VIRTUAL_ADDRESS_ERROR)
    }

    pub(crate) fn u16(bytes: &[u8], offset: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(Self::array(bytes, offset)?))
    }

    pub(crate) fn u32(bytes: &[u8], offset: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(Self::array(bytes, offset)?))
    }

    pub(crate) fn u64(bytes: &[u8], offset: usize) -> Result<u64> {
        Ok(u64::from_le_bytes(Self::array(bytes, offset)?))
    }

    pub(crate) fn i64(bytes: &[u8], offset: usize) -> Result<i64> {
        Ok(i64::from_le_bytes(Self::array(bytes, offset)?))
    }

    pub(crate) fn range(bytes: &[u8], offset: usize, size: usize) -> Result<&[u8]> {
        let end = offset
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!(ELF_BOUNDS_ERROR))?;
        bytes
            .get(offset..end)
            .ok_or_else(|| anyhow::anyhow!(ELF_BOUNDS_ERROR))
    }

    fn array<const SIZE: usize>(bytes: &[u8], offset: usize) -> Result<[u8; SIZE]> {
        Ok(Self::range(bytes, offset, SIZE)?
            .try_into()
            .map_err(|_| anyhow::anyhow!(ELF_BOUNDS_ERROR))?)
    }
}
