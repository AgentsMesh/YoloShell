use crate::{guarded, memory, types};

const MAX_STARTUP_FRAME_BYTES: usize = yolo_startup_guard::MAX_ENCODED_FRAME_BYTES;

#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_startup_maximum_frame_bytes() -> usize { MAX_STARTUP_FRAME_BYTES }

/// Returns the complete private startup frame, including its network-order length.
/// Output must point to writable storage owned by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_startup_encode_commit(process_id: u32, output: *mut types::Buffer) -> i32 {
    guarded(|| {
        if output.is_null() { return Err(()); }
        unsafe { output.write(types::Buffer::default()); }
        let bytes = yolo_startup_guard::encode_commit(process_id).map_err(|_| ())?;
        unsafe { output.write(memory::owned(bytes)); }
        Ok(())
    })
}

/// Input must reference readable memory for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_startup_validate_committed(bytes: *const u8, length: usize, process_id: u32) -> i32 {
    guarded(|| {
        let frame = unsafe { memory::bytes(types::Text { data: bytes, length }, MAX_STARTUP_FRAME_BYTES)? };
        yolo_startup_guard::validate_committed(frame, process_id).map_err(|_| ())
    })
}
