use super::HANDLE_CAPACITY;
use crate::Error;
use serde::{
    Deserialize, Deserializer, Serialize, Serializer,
    de::{SeqAccess, Visitor},
};
use std::fmt;

const INVALID_HANDLE_TYPE: i32 = 255;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Handle {
    pub kind: i32,
    pub bytes: Bytes,
}

impl Handle {
    pub(crate) fn new(kind: i32, bytes: &[u8]) -> Result<Self, Error> {
        let value = Self {
            kind,
            bytes: Bytes::new(bytes)?,
        };
        value.validate()?;
        Ok(value)
    }
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.kind <= i32::default() || self.kind == INVALID_HANDLE_TYPE {
            return Err(Error::Invalid("Linux handle type"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Bytes {
    bytes: [u8; HANDLE_CAPACITY],
    length: usize,
}

impl Bytes {
    fn new(source: &[u8]) -> Result<Self, Error> {
        if source.is_empty() || source.len() > HANDLE_CAPACITY {
            return Err(Error::Invalid("Linux handle length"));
        }
        let mut bytes = [u8::default(); HANDLE_CAPACITY];
        bytes[..source.len()].copy_from_slice(source);
        Ok(Self {
            bytes,
            length: source.len(),
        })
    }
}

impl Serialize for Bytes {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.bytes[..self.length].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Bounded;
        impl<'de> Visitor<'de> for Bounded {
            type Value = Bytes;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "between 1 and {HANDLE_CAPACITY} handle bytes")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut values: A) -> Result<Bytes, A::Error> {
                let mut bytes = [u8::default(); HANDLE_CAPACITY];
                let mut length = usize::default();
                while let Some(byte) = values.next_element::<u8>()? {
                    if length == HANDLE_CAPACITY {
                        return Err(serde::de::Error::custom("Linux handle exceeds capacity"));
                    }
                    bytes[length] = byte;
                    length += 1;
                }
                Bytes::new(&bytes[..length]).map_err(serde::de::Error::custom)
            }
        }
        deserializer.deserialize_seq(Bounded)
    }
}
