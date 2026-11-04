pub const MAX_REPLY_COUNT: usize = 16;
pub const MAX_REPLY_BYTES: usize = 96;
pub(crate) const STATUS_QUERY: u16 = 5;
pub(crate) const CURSOR_QUERY: u16 = 6;
pub(crate) const DEFAULT_ATTRIBUTE_QUERY: u16 = 0;
pub(crate) const STATUS_FINAL: char = 'n';
pub(crate) const ATTRIBUTE_FINAL: char = 'c';
pub(crate) const STATUS_REPLY: &[u8] = b"\x1b[0n";
// VT100 with no optional hardware capabilities; no advanced features claimed.
pub(crate) const ATTRIBUTE_REPLY: &[u8] = b"\x1b[?1;0c";
pub(crate) const CURSOR_REPLY_PREFIX: &str = "\x1b[";
pub(crate) const CURSOR_REPLY_SEPARATOR: char = ';';
pub(crate) const CURSOR_REPLY_FINAL: char = 'R';
