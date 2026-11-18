/// Maximum cells in either frozen grid or selection snapshot.
pub const MAX_SELECTION_CELLS: usize = 65_536;
/// Leaves room for Base64 inside common 64 KiB OSC command limits.
pub const MAX_SELECTION_BYTES: usize = 32_768;
pub(crate) const CELL_STEP: u16 = 1;
pub(crate) const SPACE: &str = " ";
pub(crate) const NEWLINE: &str = "\n";
pub(crate) const CLIPBOARD_PREFIX: &[u8] = b"\x1b]52;c;";
pub(crate) const CLIPBOARD_END: &[u8] = b"\x1b\\";
pub(crate) const ROW_PREFIX: &str = "\x1b[0m\x1b[";
pub(crate) const ROW_SUFFIX: &str = ";1H";
pub(crate) const HIDE_CURSOR: &[u8] = b"\x1b[?25l";
pub(crate) const EMPTY_VIEWPORT: &str = "There is no visible terminal text to select";
pub(crate) const CAPTURE_LIMIT: &str = "The visible terminal is too large to select";
pub(crate) const INVALID_POINT: &str = "The selection is outside the visible terminal";
pub(crate) const COPY_LIMIT: &str = "Select less text to copy to the terminal clipboard";
pub(crate) const INCOMPLETE: &str = "Release the selection before copying";
pub(crate) const EMPTY_SELECTION: &str = "The selection contains no text";
pub(crate) const BLANK_CELL_EXPECTED: &str = "a nonempty blank terminal has its origin cell";
