use crate::reader::Reader;
use anyhow::{Result, ensure};

const DYNAMIC_ENTRY_BYTES: usize = 16;
const DYNAMIC_TAG_OFFSET: usize = 0;
const DYNAMIC_VALUE_OFFSET: usize = 8;
const DYNAMIC_NULL_TAG: i64 = 0;
const DYNAMIC_STRING_TABLE_TAG: i64 = 5;
const DYNAMIC_STRING_SIZE_TAG: i64 = 10;
const DYNAMIC_VERSION_NEED_TAG: i64 = 0x6fff_fffe;
const DYNAMIC_VERSION_NEED_COUNT_TAG: i64 = 0x6fff_ffff;
const MAX_DYNAMIC_ENTRY_COUNT: usize = 4_096;
const DYNAMIC_TABLE_ERROR: &str = "Linux runtime ELF dynamic table is invalid";
const DYNAMIC_VALUE_ERROR: &str = "Linux runtime ELF dynamic value is duplicated";
const VERSION_NEED_ERROR: &str = "Linux runtime ELF version needs are invalid";

#[derive(Default)]
struct DynamicValues {
    strings: Option<u64>,
    string_size: Option<u64>,
    version_needs: Option<u64>,
    version_need_count: Option<u64>,
}

pub(crate) fn verify(bytes: &[u8]) -> Result<()> {
    let reader = Reader::new(bytes)?;
    let segments = reader.segments()?;
    let mut dynamic_segments = segments
        .iter()
        .copied()
        .filter(|segment| segment.is_dynamic());
    let Some(dynamic_segment) = dynamic_segments.next() else {
        return Ok(());
    };
    ensure!(dynamic_segments.next().is_none(), DYNAMIC_TABLE_ERROR);
    let values = dynamic_values(reader.segment_bytes(dynamic_segment)?)?;
    let count = usize::try_from(required(values.version_need_count)?)?;
    ensure!(count > 0, VERSION_NEED_ERROR);
    let string_size = usize::try_from(required(values.string_size)?)?;
    ensure!(string_size > 0, VERSION_NEED_ERROR);
    let strings = reader.mapped(&segments, required(values.strings)?)?;
    let strings = Reader::range(strings, 0, string_size)?;
    let version_needs = reader.mapped(&segments, required(values.version_needs)?)?;
    crate::versions::verify(version_needs, count, strings)
}

fn required(value: Option<u64>) -> Result<u64> {
    value.ok_or_else(|| anyhow::anyhow!(VERSION_NEED_ERROR))
}

fn dynamic_values(bytes: &[u8]) -> Result<DynamicValues> {
    ensure!(
        bytes.len() % DYNAMIC_ENTRY_BYTES == 0
            && bytes.len() / DYNAMIC_ENTRY_BYTES <= MAX_DYNAMIC_ENTRY_COUNT,
        DYNAMIC_TABLE_ERROR
    );
    let mut values = DynamicValues::default();
    let mut terminated = false;
    for entry in bytes.chunks_exact(DYNAMIC_ENTRY_BYTES) {
        let tag = Reader::i64(entry, DYNAMIC_TAG_OFFSET)?;
        if tag == DYNAMIC_NULL_TAG {
            terminated = true;
            break;
        }
        let value = Reader::u64(entry, DYNAMIC_VALUE_OFFSET)?;
        match tag {
            DYNAMIC_STRING_TABLE_TAG => set_once(&mut values.strings, value)?,
            DYNAMIC_STRING_SIZE_TAG => set_once(&mut values.string_size, value)?,
            DYNAMIC_VERSION_NEED_TAG => set_once(&mut values.version_needs, value)?,
            DYNAMIC_VERSION_NEED_COUNT_TAG => set_once(&mut values.version_need_count, value)?,
            _ => {}
        }
    }
    ensure!(terminated, DYNAMIC_TABLE_ERROR);
    Ok(values)
}

fn set_once(slot: &mut Option<u64>, value: u64) -> Result<()> {
    ensure!(slot.replace(value).is_none(), DYNAMIC_VALUE_ERROR);
    Ok(())
}
