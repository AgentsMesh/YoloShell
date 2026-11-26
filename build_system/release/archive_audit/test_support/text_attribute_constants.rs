pub const MIXED_PAYLOAD: &[u8] = b"first\r\nsecond\nembedded\0nul\r\nlast\r";
pub const ARCHIVE_FILE: &str = "text-attribute.zip";
pub const EXTRACTION_DIRECTORY: &str = "extracted";
pub const RAW_DEFLATE_FINAL_STORED_BLOCK: u8 = 0x01;
pub const EXTRACTION_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
pub const PROCESS_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(10);
pub const EXTRACTION_TIMEOUT_MESSAGE: &str = "ditto extraction exceeded its deadline";
