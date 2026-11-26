//! Bounded Cap'n Proto serialization for structural domain documents.
mod budget;
mod constants;
mod pending;
mod read;
mod write;

use capnp::{
    message::{Builder, ReaderOptions},
    serialize,
};
use constants::*;
pub use constants::{CAPNP_NESTING_LIMIT, MAX_MESSAGE_BYTES, MAX_VALUE_DEPTH, SCHEMA_VERSION};
use serde::{Serialize, de::DeserializeOwned};
use yolo_capnp_schema::value_capnp::document;
pub use yolo_capnp_value::{Value, ValueError, from_value, to_value};

pub fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, ValueError> {
    encode_value(&to_value(value)?)
}

pub fn encode_value(value: &Value) -> Result<Vec<u8>, ValueError> {
    let mut message = Builder::new_default();
    let mut root = message.init_root::<document::Builder<'_>>();
    root.set_version(SCHEMA_VERSION);
    write_value(root.init_value(), value)?;
    let bytes = serialize::write_message_to_words(&message);
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(ValueError::Limit);
    }
    Ok(bytes)
}

/// Budget the complete structural value, reserving envelope space inside the 16 MiB wire limit.
pub fn write_value(
    mut builder: yolo_capnp_schema::value_capnp::value::Builder<'_>,
    source: &Value,
) -> Result<(), ValueError> {
    write::value(
        builder.reborrow(),
        source,
        0,
        &mut budget::Budget::default(),
    )?;
    let size = builder.into_reader().total_size().map_err(error)?;
    if size.word_count > (budget::MAX_VALUE_BYTES / WORD_BYTES) as u64 {
        return Err(ValueError::Limit);
    }
    Ok(())
}

pub fn read_value(
    reader: yolo_capnp_schema::value_capnp::value::Reader<'_>,
) -> Result<Value, ValueError> {
    read::value(reader, 0, &mut budget::Budget::default(), |_| {})
}

/// Canonical, unpacked root bytes for signatures. Never signs segmentation or transport framing.
pub fn canonical<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, ValueError> {
    let value = to_value(value)?;
    let mut builder = Builder::new_default();
    let mut root = builder.init_root::<document::Builder<'_>>();
    root.set_version(SCHEMA_VERSION);
    write_value(root.init_value(), &value)?;
    let reader = builder.into_reader();
    let canonical = reader.canonicalize().map_err(error)?;
    let bytes = capnp::Word::words_to_bytes(&canonical);
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(ValueError::Limit);
    }
    Ok(bytes.to_vec())
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ValueError> {
    from_value(decode_value(bytes)?)
}

pub fn decode_with_depth<T: DeserializeOwned>(
    bytes: &[u8],
    maximum: usize,
) -> Result<T, ValueError> {
    let value = decode_value(bytes)?;
    let mut pending = vec![(&value, 0)];
    while let Some((value, depth)) = pending.pop() {
        if depth > maximum {
            return Err(ValueError::Depth);
        }
        match value {
            Value::List(values) => {
                pending.extend(values.iter().map(|value| (value, depth + CHILD_DEPTH)))
            }
            Value::Map(values) => {
                for (key, value) in values {
                    pending.extend([(key, depth + CHILD_DEPTH), (value, depth + CHILD_DEPTH)]);
                }
            }
            _ => {}
        }
    }
    from_value(value)
}

pub fn decode_value(bytes: &[u8]) -> Result<Value, ValueError> {
    decode_value_with_cleanup(bytes, |_| {})
}

/// Run cleanup on every partially constructed subtree if decoding fails.
/// The callback must not panic. The caller owns cleanup of the successful result.
pub fn decode_value_with_cleanup(
    bytes: &[u8],
    cleanup: fn(&mut Value),
) -> Result<Value, ValueError> {
    if bytes.is_empty() || bytes.len() > MAX_MESSAGE_BYTES {
        return Err(ValueError::Limit);
    }
    let mut remaining = bytes;
    let options = ReaderOptions {
        traversal_limit_in_words: Some(MAX_MESSAGE_BYTES / WORD_BYTES),
        nesting_limit: CAPNP_NESTING_LIMIT,
    };
    let message =
        serialize::read_message_from_flat_slice(&mut remaining, options).map_err(error)?;
    if !remaining.is_empty() {
        return Err(ValueError::Trailing);
    }
    let root = message.get_root::<document::Reader<'_>>().map_err(error)?;
    if root.get_version() != SCHEMA_VERSION {
        return Err(ValueError::Schema);
    }
    read::value(
        root.get_value().map_err(error)?,
        0,
        &mut budget::Budget::default(),
        cleanup,
    )
}

fn error(error: impl std::fmt::Display) -> ValueError {
    ValueError::Invalid(error.to_string())
}
