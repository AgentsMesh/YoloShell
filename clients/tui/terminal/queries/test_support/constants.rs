pub const ROWS: u16 = 4;
pub const COLUMNS: u16 = 8;
pub const SCROLLBACK: usize = 16;
pub const DSR: &[u8] = b"\x1b[5n";
pub const CPR: &[u8] = b"\x1b[6n";
pub const DA: &[u8] = b"\x1b[c";
pub const DA_ZERO: &[u8] = b"\x1b[0c";
pub const STATUS: &[u8] = b"\x1b[0n";
pub const ATTRIBUTES: &[u8] = b"\x1b[?1;0c";
pub const HOME_REPORT: &[u8] = b"\x1b[1;1R";
pub const THIRD_REPORT: &[u8] = b"\x1b[2;3R";
pub const THIRD_POSITION: &[u8] = b"\x1b[2;3H";
pub const ORDERED_QUERIES: &[u8] = b"\x1b[2;3H\x1b[6n\x1b[3;4H\x1b[6n\x1b[5n\x1b[c";
pub const FOURTH_REPORT: &[u8] = b"\x1b[3;4R";
pub const PARTIAL_CPR: &[u8] = b"\x1b[6";
pub const CPR_FINAL: &[u8] = b"n";
pub const IGNORE_QUERIES: &[&[u8]] = &[
    b"\x1b[?6n",
    b"\x1b[>0c",
    b"\x1b[1c",
    b"\x1b[n",
    b"\x1b[5;6n",
    b"\x1b[6:0n",
    b"\x1b[6 n",
    b"\x1b[6!n",
    b"\x1b[6\x18n",
    b"\x1b[6\x1an",
    b"\x1b]52;c;?\x07",
    b"\x1b]2;[6n\x07",
    b"\x1bP[6n\x1b\\",
    b"\x1b[999999999999999999999999999999n",
];
pub const PARAMETER_OVERFLOW: &[u8] =
    b"\x1b[0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0c";
pub const INTERMEDIATE_OVERFLOW: &[u8] = b"\x1b[6   n";
pub const WIDE_TEXT: &str = "界";
pub const FILL_ROW: &[u8] = b"abcdefgh";
pub const LAST_COLUMN_REPORT: &[u8] = b"\x1b[1;8R";
pub const ORIGIN_QUERY: &[u8] = b"\x1b[2;4r\x1b[?6h\x1b[2;3H\x1b[6n";
pub const SAVED_ORIGIN_QUERY: &[u8] = b"\x1b[2;4r\x1b[?6h\x1b[2;3H\x1b7\x1b[?6l\x1b8\x1b[6n";
pub const SCROLLBACK_QUERY: &[u8] = b"a\r\nb\r\nc\r\nd\r\ne\x1b[2;3H";
pub const OUTPUT_SUFFIX: &[u8] = b"tail";
pub const OUTPUT_SUFFIX_TEXT: &str = "tail";
pub const SPLIT_STEP: usize = 1;
pub const CANCEL_SEQUENCE: &[u8] = b"\x18";
pub const SUBSTITUTE_SEQUENCE: &[u8] = b"\x1a";
pub const RESET_SCREEN: &[u8] = b"\x1bc";
pub const UNMATCHED_FINAL_TEXT: &str = "n";
