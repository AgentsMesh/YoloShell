//! In-process typed C bridge for the private Cap'n Proto relay protocol. No JSON conversion.
mod constants;
mod memory;
mod outcome;
mod request;
mod startup;
pub mod types;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;

use constants::*;
use std::panic::{AssertUnwindSafe, catch_unwind};
use yolo_hub_relay_control_protocol as protocol;

fn guarded(action: impl FnOnce() -> Result<(), ()>) -> i32 {
    match catch_unwind(AssertUnwindSafe(action)) { Ok(Ok(())) => OK, Ok(Err(())) => INVALID, Err(_) => PANIC }
}

/// Pointers must reference live, non-overlapping structs for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_relay_encode_request(input: *const types::Request, output: *mut types::Buffer) -> i32 {
    guarded(|| {
        if input.is_null() || output.is_null() { return Err(()); }
        unsafe { output.write(types::Buffer::default()); }
        let request = unsafe { request::decode(&*input)? };
        let bytes = protocol::encode_request(&request).map_err(|_| ())?;
        unsafe { output.write(memory::owned(bytes)); }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_relay_encode_response(input: *const types::Request, result: *const types::Outcome, output: *mut types::Buffer) -> i32 {
    guarded(|| {
        if input.is_null() || result.is_null() || output.is_null() { return Err(()); }
        unsafe { output.write(types::Buffer::default()); }
        let request = unsafe { request::decode(&*input)? };
        let response = protocol::Response::new(&request, outcome::decode(unsafe { &*result }, &request)?).map_err(|_| ())?;
        let bytes = protocol::encode_response(&response).map_err(|_| ())?;
        unsafe { output.write(memory::owned(bytes)); }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_relay_encode_status(kind: u32, holder: types::Text, output: *mut types::Buffer) -> i32 {
    guarded(|| {
        if output.is_null() { return Err(()); }
        unsafe { output.write(types::Buffer::default()); }
        let text = unsafe { memory::text(holder, protocol::MAX_HOLDER_BYTES)? };
        let status = protocol::Status {
            kind: protocol::StatusKind::try_from(u16::try_from(kind).map_err(|_| ())?).map_err(|_| ())?,
            holder_id: (!text.is_empty()).then(|| text.to_owned()),
        };
        unsafe { output.write(memory::owned(protocol::encode_status(&status).map_err(|_| ())?)); }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_relay_decode(bytes: *const u8, length: usize, output: *mut types::Decoded) -> i32 {
    guarded(|| {
        if output.is_null() { return Err(()); }
        unsafe { output.write(types::Decoded::default()); }
        let data = unsafe { memory::bytes(types::Text { data: bytes, length }, protocol::MAX_FRAME_BYTES)? };
        let mut result = types::Decoded::default();
        if let Ok(decoded) = protocol::decode_request(data) {
            result.kind = REQUEST; request::encode(&decoded, &mut result);
        } else if let Ok(decoded) = protocol::decode_response(data) {
            result.kind = RESPONSE;
            let fields = decoded.fields();
            let request = protocol::Request::new(protocol::RequestFields { schema_version: fields.schema_version,
                launch_id: fields.launch_id, terminal: fields.terminal, request: fields.request.clone() }).map_err(|_| ())?;
            request::encode(&request, &mut result);
            result.outcome = outcome::encode(fields.outcome);
        } else {
            let status = protocol::decode_status(data).map_err(|_| ())?;
            result.kind = STATUS; result.status = status.kind as u32;
            result.holder = memory::owned(status.holder_id.unwrap_or_default().into_bytes());
        }
        unsafe { output.write(result); }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_relay_release_buffer(value: *mut types::Buffer) {
    if !value.is_null() { unsafe { memory::release(&mut *value); } }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_relay_release_decoded(value: *mut types::Decoded) {
    if !value.is_null() {
        unsafe { memory::release(&mut (*value).prompt); memory::release(&mut (*value).holder); value.write(types::Decoded::default()); }
    }
}
