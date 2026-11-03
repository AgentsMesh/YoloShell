use crate::constants::*;
use yolo_capnp_codec::Value;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_tag(value: *const Value) -> u32 {
    match unsafe { value.as_ref() } {
        Some(Value::Null) => NULL,
        Some(Value::Boolean(_)) => BOOLEAN,
        Some(Value::Signed(_)) => SIGNED,
        Some(Value::Unsigned(_)) => UNSIGNED,
        Some(Value::Float(_)) => FLOAT,
        Some(Value::Text(_)) => TEXT,
        Some(Value::Data(_)) => DATA,
        Some(Value::List(_)) => LIST,
        Some(Value::Map(_)) => MAP,
        None => INVALID_TAG,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_bool(value: *const Value) -> bool {
    matches!(unsafe { value.as_ref() }, Some(Value::Boolean(true)))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_i64(value: *const Value) -> i64 {
    match unsafe { value.as_ref() } {
        Some(Value::Signed(value)) => *value,
        _ => 0,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_u64(value: *const Value) -> u64 {
    match unsafe { value.as_ref() } {
        Some(Value::Unsigned(value)) => *value,
        _ => 0,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_f64(value: *const Value) -> f64 {
    match unsafe { value.as_ref() } {
        Some(Value::Float(value)) => *value,
        _ => 0.0,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_bytes(
    value: *const Value,
    length: *mut usize,
) -> *const u8 {
    if length.is_null() {
        return std::ptr::null();
    }
    unsafe {
        length.write(0);
    }
    let bytes = match unsafe { value.as_ref() } {
        Some(Value::Text(value)) => value.as_bytes(),
        Some(Value::Data(value)) => value,
        _ => return std::ptr::null(),
    };
    unsafe {
        length.write(bytes.len());
    }
    bytes.as_ptr()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_count(value: *const Value) -> usize {
    match unsafe { value.as_ref() } {
        Some(Value::List(value)) => value.len(),
        Some(Value::Map(value)) => value.len(),
        _ => 0,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_child(
    value: *const Value,
    index: usize,
) -> *const Value {
    match unsafe { value.as_ref() } {
        Some(Value::List(value)) => value
            .get(index)
            .map_or(std::ptr::null(), std::ptr::from_ref),
        _ => std::ptr::null(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_key(
    value: *const Value,
    index: usize,
) -> *const Value {
    match unsafe { value.as_ref() } {
        Some(Value::Map(value)) => value.get(index).map_or(std::ptr::null(), |entry| &entry.0),
        _ => std::ptr::null(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_get_value(
    value: *const Value,
    index: usize,
) -> *const Value {
    match unsafe { value.as_ref() } {
        Some(Value::Map(value)) => value.get(index).map_or(std::ptr::null(), |entry| &entry.1),
        _ => std::ptr::null(),
    }
}
