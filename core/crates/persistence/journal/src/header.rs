use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct JournalHeader {
    pub(super) generation: Uuid,
    pub(super) sealed_bytes: u64,
}

impl JournalHeader {
    pub(super) fn new(sealed_bytes: u64) -> Self {
        Self {
            generation: Uuid::new_v4(),
            sealed_bytes,
        }
    }

    pub(super) fn encode(self) -> [u8; JOURNAL_HEADER_BYTES] {
        let mut bytes = [0_u8; JOURNAL_HEADER_BYTES];
        bytes[..MAGIC_BYTES].copy_from_slice(JOURNAL_HEADER_MAGIC);
        bytes[GENERATION_START..GENERATION_END].copy_from_slice(self.generation.as_bytes());
        bytes[DURABLE_LENGTH_START..DURABLE_LENGTH_END]
            .copy_from_slice(&self.sealed_bytes.to_be_bytes());
        let checksum = crc32fast::hash(&bytes[..METADATA_CHECKSUM_START]);
        bytes[METADATA_CHECKSUM_START..METADATA_CHECKSUM_END]
            .copy_from_slice(&checksum.to_be_bytes());
        bytes
    }

    pub(super) fn decode(bytes: &[u8]) -> Result<Self, JournalError> {
        if bytes.len() != JOURNAL_HEADER_BYTES || &bytes[..MAGIC_BYTES] != JOURNAL_HEADER_MAGIC {
            return Err(JournalError::InvalidFormat);
        }
        let stored = read_u32(&bytes[METADATA_CHECKSUM_START..METADATA_CHECKSUM_END])?;
        if crc32fast::hash(&bytes[..METADATA_CHECKSUM_START]) != stored {
            return Err(JournalError::InvalidFormat);
        }
        let generation = Uuid::from_slice(&bytes[GENERATION_START..GENERATION_END])
            .map_err(|_| JournalError::InvalidFormat)?;
        let sealed_bytes = read_u64(&bytes[DURABLE_LENGTH_START..DURABLE_LENGTH_END])?;
        if sealed_bytes < JOURNAL_HEADER_BYTES as u64 {
            return Err(JournalError::InvalidFormat);
        }
        Ok(Self {
            generation,
            sealed_bytes,
        })
    }
}

pub(super) fn read_header(file: &mut File) -> Result<JournalHeader, JournalError> {
    file.seek(SeekFrom::Start(0))?;
    let mut bytes = [0_u8; JOURNAL_HEADER_BYTES];
    file.read_exact(&mut bytes).map_err(map_header_read)?;
    JournalHeader::decode(&bytes)
}

pub(super) fn write_header(file: &mut File, header: JournalHeader) -> Result<(), JournalError> {
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&header.encode())?;
    file.flush()?;
    Ok(())
}

fn map_header_read(error: std::io::Error) -> JournalError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        JournalError::InvalidFormat
    } else {
        JournalError::Io(error)
    }
}

fn read_u64(bytes: &[u8]) -> Result<u64, JournalError> {
    bytes
        .try_into()
        .map(u64::from_be_bytes)
        .map_err(|_| JournalError::InvalidFormat)
}

fn read_u32(bytes: &[u8]) -> Result<u32, JournalError> {
    bytes
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| JournalError::InvalidFormat)
}
