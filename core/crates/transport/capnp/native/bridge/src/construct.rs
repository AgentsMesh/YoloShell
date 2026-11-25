use crate::{constants::*, guarded, memory};
use yolo_capnp_codec::Value;

#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_value_null() -> *mut Value {
    Box::into_raw(Box::new(Value::Null))
}
#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_value_boolean(value: bool) -> *mut Value {
    Box::into_raw(Box::new(Value::Boolean(value)))
}
#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_value_signed(value: i64) -> *mut Value {
    Box::into_raw(Box::new(Value::Signed(value)))
}
#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_value_unsigned(value: u64) -> *mut Value {
    Box::into_raw(Box::new(Value::Unsigned(value)))
}
#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_value_float(value: f64) -> *mut Value {
    Box::into_raw(Box::new(Value::Float(value)))
}
#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_value_list() -> *mut Value {
    Box::into_raw(Box::new(Value::List(Vec::new())))
}
#[unsafe(no_mangle)]
pub extern "C" fn yolo_native_value_map() -> *mut Value {
    Box::into_raw(Box::new(Value::Map(Vec::new())))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_text(data: *const u8, length: usize) -> *mut Value {
    match unsafe { memory::bytes(data, length) }
        .ok()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    {
        Some(text) => Box::into_raw(Box::new(Value::Text(text.into()))),
        None => std::ptr::null_mut(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_data(data: *const u8, length: usize) -> *mut Value {
    match unsafe { memory::bytes(data, length) } {
        Ok(bytes) => Box::into_raw(Box::new(Value::Data(bytes.into()))),
        Err(()) => std::ptr::null_mut(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_push(parent: *mut Value, child: *mut Value) -> i32 {
    guarded(|| {
        if parent.is_null() || child.is_null() || parent == child {
            return Err(());
        }
        let Value::List(values) = (unsafe { &mut *parent }) else {
            return Err(());
        };
        if values.len() >= MAX_CONTAINER_ITEMS {
            return Err(());
        }
        values.push(*unsafe { Box::from_raw(child) });
        Ok(())
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn yolo_native_value_insert(
    parent: *mut Value,
    key: *mut Value,
    value: *mut Value,
) -> i32 {
    guarded(|| {
        if parent.is_null()
            || key.is_null()
            || value.is_null()
            || parent == key
            || parent == value
            || key == value
        {
            return Err(());
        }
        let Value::Map(values) = (unsafe { &mut *parent }) else {
            return Err(());
        };
        if values.len() >= MAX_CONTAINER_ITEMS
            || values.iter().any(|entry| &entry.0 == unsafe { &*key })
        {
            return Err(());
        }
        values.push((*unsafe { Box::from_raw(key) }, *unsafe {
            Box::from_raw(value)
        }));
        Ok(())
    })
}
