pub const SGR: &[u8] = b"\x1b[?1000h\x1b[?1006h";
pub const DRAG: &[u8] = b"\x1b[?1002h\x1b[?1006h";
pub const MOTION: &[u8] = b"\x1b[?1003h\x1b[?1006h";
pub const X10: &[u8] = b"\x1b[?9h";
pub const DEFAULT: &[u8] = b"\x1b[?1000h";
pub const UTF8: &[u8] = b"\x1b[?1000h\x1b[?1005h";
pub const SGR_LEFT: &[u8] = b"\x1b[<0;1;1M";
pub const SGR_MIDDLE: &[u8] = b"\x1b[<1;1;1M";
pub const SGR_RIGHT: &[u8] = b"\x1b[<2;1;1M";
pub const SGR_RELEASE: &[u8] = b"\x1b[<0;1;1m";
pub const SGR_MODIFIED: &[u8] = b"\x1b[<28;1;1M";
pub const SGR_DRAG: &[u8] = b"\x1b[<32;1;1M";
pub const SGR_MOTION: &[u8] = b"\x1b[<35;1;1M";
pub const WHEELS: [&[u8]; 4] = [
    b"\x1b[<64;1;1M",
    b"\x1b[<65;1;1M",
    b"\x1b[<66;1;1M",
    b"\x1b[<67;1;1M",
];
pub const LEGACY_LEFT: &[u8] = b"\x1b[M !!";
pub const LEGACY_RELEASE: &[u8] = b"\x1b[M#!!";
pub const LEGACY_EDGE: &[u8] = b"\x1b[M \xff!";
pub const UTF8_EDGE: &[u8] = b"\x1b[M \xdf\xbf!";
pub const SGR_EDGE: &[u8] = b"\x1b[<0;65535;1M";
pub const LEGACY_LAST_COLUMN: u16 = 222;
pub const UTF8_LAST_COLUMN: u16 = 2014;
pub const CELL: u16 = 1;
pub const TEST_COLUMNS: u16 = 10;
