use crate::types::{Buffer, Text};

pub(super) unsafe fn bytes<'a>(value: Text, maximum: usize) -> Result<&'a [u8], ()> {
    if value.length > maximum { return Err(()); }
    if value.length == 0 { return Ok(&[]); }
    if value.data.is_null() { return Err(()); }
    Ok(unsafe { std::slice::from_raw_parts(value.data, value.length) })
}
pub(super) unsafe fn text<'a>(value: Text, maximum: usize) -> Result<&'a str, ()> {
    std::str::from_utf8(unsafe { bytes(value, maximum)? }).map_err(|_| ())
}
pub(super) fn owned(value: Vec<u8>) -> Buffer {
    if value.is_empty() { return Buffer::default(); }
    let mut bytes = value.into_boxed_slice();
    let result = Buffer { data: bytes.as_mut_ptr(), length: bytes.len() };
    std::mem::forget(bytes);
    result
}
pub(super) unsafe fn release(value: &mut Buffer) {
    if !value.data.is_null() {
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(value.data, value.length)) });
    }
    *value = Buffer::default();
}
