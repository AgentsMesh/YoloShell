#[derive(Clone, Debug)]
pub(crate) struct EndRecord {
    pub entries: usize,
    pub central_offset: u64,
    pub central_size: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct CentralEntry {
    pub version_needed: u16,
    pub flags: u16,
    pub method: u16,
    pub modified_time: u16,
    pub modified_date: u16,
    pub crc32: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub name: Vec<u8>,
    pub path: String,
    pub mode: u16,
    pub is_directory: bool,
    pub local_offset: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct EntrySpan {
    pub start: u64,
    pub end: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    pub path: String,
    pub mode: u16,
    pub is_directory: bool,
    pub crc32: u32,
    pub sha256: String,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZipInventory {
    pub entries: Vec<ArchiveEntry>,
}
