//! Cap'n Proto documents whose owned values, builder segments and output are scrubbed.
mod allocator;
mod normalize;
use capnp::{
    message::{Builder, ReaderOptions},
    serialize,
};
use yolo_capnp_codec::{CAPNP_NESTING_LIMIT, MAX_MESSAGE_BYTES, SCHEMA_VERSION, Value, ValueError};
use yolo_capnp_schema::value_capnp::{document, value};
use zeroize::{Zeroize, Zeroizing};

const WORD_BYTES: usize = std::mem::size_of::<capnp::Word>();
const INVALID: &str = "invalid private Cap'n Proto document";

pub struct OwnedValue(pub Value);
impl OwnedValue {
    pub fn into_inner(mut self) -> Value {
        std::mem::replace(&mut self.0, Value::Null)
    }
}
impl Drop for OwnedValue {
    fn drop(&mut self) {
        erase(&mut self.0);
    }
}

pub fn erase(value: &mut Value) {
    match value {
        Value::Text(value) => value.zeroize(),
        Value::Data(value) => value.zeroize(),
        Value::Boolean(value) => value.zeroize(),
        Value::Signed(value) => value.zeroize(),
        Value::Unsigned(value) => value.zeroize(),
        Value::Float(value) => value.zeroize(),
        Value::List(values) => {
            for value in values.iter_mut() {
                erase(value);
            }
            values.clear();
        }
        Value::Map(values) => {
            for (key, value) in values.iter_mut() {
                erase(key);
                erase(value);
            }
            values.clear();
        }
        _ => {}
    }
}

/// Scrub incomplete parse trees and rejected duplicates before relinquishing their storage.
pub fn decode(bytes: &[u8]) -> Result<OwnedValue, ValueError> {
    let mut value = OwnedValue(yolo_capnp_codec::decode_value_with_cleanup(bytes, erase)?);
    normalize::unique(&mut value.0)?;
    Ok(value)
}

pub fn encode(value: &Value) -> Result<Zeroizing<Vec<u8>>, ValueError> {
    let mut builder = Builder::new(allocator::Scrubbed::default());
    let mut document = builder.init_root::<document::Builder<'_>>();
    document.set_version(SCHEMA_VERSION);
    yolo_capnp_codec::write_value(document.init_value(), value)?;
    let bytes = Zeroizing::new(serialize::write_message_to_words(&builder));
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(ValueError::Limit);
    }
    Ok(bytes)
}

/// Borrow the wire contents directly. Callers copy secrets only into scrubbing owners.
pub fn read<T>(
    bytes: &[u8],
    action: impl FnOnce(value::Reader<'_>) -> Result<T, ValueError>,
) -> Result<T, ValueError> {
    if bytes.is_empty() || bytes.len() > MAX_MESSAGE_BYTES {
        return Err(ValueError::Limit);
    }
    let mut remaining = bytes;
    let message = serialize::read_message_from_flat_slice(
        &mut remaining,
        ReaderOptions {
            traversal_limit_in_words: Some(MAX_MESSAGE_BYTES / WORD_BYTES),
            nesting_limit: CAPNP_NESTING_LIMIT,
        },
    )
    .map_err(|_| invalid())?;
    if !remaining.is_empty() {
        return Err(ValueError::Trailing);
    }
    let root = message
        .get_root::<document::Reader<'_>>()
        .map_err(|_| invalid())?;
    if root.get_version() != SCHEMA_VERSION {
        return Err(ValueError::Schema);
    }
    action(root.get_value().map_err(|_| invalid())?)
}

pub fn invalid() -> ValueError {
    ValueError::Invalid(INVALID.into())
}
