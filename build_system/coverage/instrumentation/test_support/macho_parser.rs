use super::{Image, constants::*, geometry, reader::Reader, sections};

pub fn inspect(bytes: &[u8]) -> Result<Image, String> {
    let reader = Reader(bytes);
    if reader.word(0)? != MAGIC_64_LE {
        return Err("only thin 64-bit little-endian Mach-O images are supported".into());
    }
    reader.slice(0, HEADER_BYTES)?;
    let count = usize::try_from(reader.word(COMMAND_COUNT_OFFSET)?)
        .map_err(|_| "Mach-O load-command count exceeds platform limits")?;
    let size = usize::try_from(reader.word(COMMAND_BYTES_OFFSET)?)
        .map_err(|_| "Mach-O load-command bytes exceed platform limits")?;
    if count == 0 || count > MAX_LOAD_COMMANDS {
        return Err("invalid or excessive Mach-O load-command count".into());
    }
    if count
        .checked_mul(COMMAND_HEADER_BYTES)
        .ok_or("Mach-O command count overflow")?
        > size
    {
        return Err("Mach-O command count exceeds declared load-command bytes".into());
    }
    let commands = Reader(reader.slice(HEADER_BYTES, size)?);
    let mut image = Image {
        segments: Vec::new(),
        file_size: u64::try_from(bytes.len()).map_err(|_| "Mach-O input is too large")?,
    };
    let mut offset = 0usize;
    let mut section_count = 0usize;
    for _ in 0..count {
        let command = Reader(commands.slice(offset, COMMAND_HEADER_BYTES)?);
        let kind = command.word(0)?;
        if kind == SEGMENT_COMMAND_32 {
            return Err("32-bit segment commands are unsupported in a 64-bit Mach-O image".into());
        }
        let length = usize::try_from(command.word(COMMAND_SIZE_OFFSET)?)
            .map_err(|_| "Mach-O load command is too large")?;
        if length < COMMAND_HEADER_BYTES || length % COMMAND_ALIGNMENT != 0 {
            return Err("invalid Mach-O load-command size or alignment".into());
        }
        let command = Reader(commands.slice(offset, length)?);
        if kind == SEGMENT_COMMAND {
            if image.segments.len() == MAX_SEGMENTS {
                return Err("excessive Mach-O segment count".into());
            }
            image
                .segments
                .push(sections::segment(&command, &mut section_count)?);
        }
        offset = offset
            .checked_add(length)
            .ok_or("Mach-O command offset overflow")?;
    }
    if offset != size {
        return Err("Mach-O load-command size does not match its command count".into());
    }
    geometry::validate(&image)?;
    Ok(image)
}
