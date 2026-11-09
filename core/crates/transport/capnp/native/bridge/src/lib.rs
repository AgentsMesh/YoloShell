//! In-process owned structural values for native clients of private Cap'n Proto IPC.
mod access;
mod constants;
mod construct;
mod memory;

pub use access::*;
use constants::*;
pub use construct::*;
pub use memory::Buffer;
use yolo_capnp_codec::Value;

fn guarded(action: impl FnOnce() -> Result<(), ()>) -> i32 {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        Ok(Ok(())) => OK,
        Ok(Err(())) => INVALID,
        Err(_) => PANIC,
    }
}

/// Input pointers must reference live memory; ownership of output transfers to the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_decode(
    input: *const u8,
    length: usize,
    output: *mut *mut Value,
) -> i32 {
    guarded(|| {
        if output.is_null() {
            return Err(());
        }
        unsafe {
            output.write(std::ptr::null_mut());
        }
        let bytes = unsafe { memory::bytes(input, length)? };
        let value = yolo_capnp_sensitive::decode(bytes).map_err(|_| ())?;
        unsafe {
            output.write(Box::into_raw(Box::new(value.into_inner())));
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_encode(input: *const Value, output: *mut Buffer) -> i32 {
    guarded(|| {
        if input.is_null() || output.is_null() {
            return Err(());
        }
        unsafe {
            output.write(Buffer::default());
        }
        let bytes = yolo_capnp_sensitive::encode(unsafe { &*input }).map_err(|_| ())?;
        unsafe {
            output.write(memory::owned(bytes));
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_release(value: *mut Value) {
    if !value.is_null() {
        let mut owned = unsafe { Box::from_raw(value) };
        yolo_capnp_sensitive::erase(&mut owned);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_release_buffer(value: *mut Buffer) {
    if !value.is_null() {
        unsafe {
            memory::release(&mut *value);
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
