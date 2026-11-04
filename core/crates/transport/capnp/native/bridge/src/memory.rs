#[repr(C)]
pub struct Buffer {
    pub data: *mut u8,
    pub length: usize,
}
impl Default for Buffer {
    fn default() -> Self {
        Self {
            data: std::ptr::null_mut(),
            length: 0,
        }
    }
}

pub(crate) unsafe fn bytes<'a>(data: *const u8, length: usize) -> Result<&'a [u8], ()> {
    if length > yolo_capnp_codec::MAX_MESSAGE_BYTES {
        return Err(());
    }
    if length == 0 {
        return Ok(&[]);
    }
    if data.is_null() {
        return Err(());
    }
    Ok(unsafe { std::slice::from_raw_parts(data, length) })
}
pub(crate) fn owned(source: zeroize::Zeroizing<Vec<u8>>) -> Buffer {
    // Shrinking a secret-bearing Vec could free its old allocation without scrubbing it.
    let mut bytes = vec![0; source.len()].into_boxed_slice();
    bytes.copy_from_slice(&source);
    let result = Buffer {
        data: bytes.as_mut_ptr(),
        length: bytes.len(),
    };
    std::mem::forget(bytes);
    result
}
pub(crate) unsafe fn release(buffer: &mut Buffer) {
    if !buffer.data.is_null() {
        use zeroize::Zeroize;
        unsafe { std::slice::from_raw_parts_mut(buffer.data, buffer.length) }.zeroize();
        drop(unsafe {
            Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                buffer.data,
                buffer.length,
            ))
        });
    }
    *buffer = Buffer::default();
}
