pub(super) fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    write(bytes, offset, value.to_le_bytes());
}

pub(super) fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    write(bytes, offset, value.to_le_bytes());
}

pub(super) fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    write(bytes, offset, value.to_le_bytes());
}

pub(super) fn write_i64(bytes: &mut [u8], offset: usize, value: i64) {
    write(bytes, offset, value.to_le_bytes());
}

fn write<const SIZE: usize>(bytes: &mut [u8], offset: usize, value: [u8; SIZE]) {
    bytes[offset..offset + SIZE].copy_from_slice(&value);
}
