use crate::{ValueError, constants::*};
use capnp::traits::HasStructSize;
use yolo_capnp_schema::value_capnp::{entry, value};

// Reserve bounded RPC call/result structs, capabilities, segment tables and landing pads.
pub(crate) const ENVELOPE_RESERVE_BYTES: usize = 4096;
pub(crate) const MAX_VALUE_BYTES: usize = MAX_MESSAGE_BYTES - ENVELOPE_RESERVE_BYTES;
const VALUE_SIZE: capnp::private::layout::StructSize = value::Builder::STRUCT_SIZE;
const ENTRY_SIZE: capnp::private::layout::StructSize = entry::Builder::STRUCT_SIZE;
const VALUE_BYTES: usize = (VALUE_SIZE.data as usize + VALUE_SIZE.pointers as usize) * WORD_BYTES;
const ENTRY_BYTES: usize = (ENTRY_SIZE.data as usize + ENTRY_SIZE.pointers as usize) * WORD_BYTES;
const TEXT_TERMINATOR_BYTES: usize = 1;
const MAP_CHILDREN: usize = 2;

pub(crate) struct Budget {
    nodes: usize,
    bytes: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            nodes: MAX_VALUE_NODES,
            bytes: MAX_VALUE_BYTES,
        }
    }
}
impl Budget {
    pub fn node(&mut self) -> Result<(), ValueError> {
        self.nodes = self.nodes.checked_sub(1).ok_or(ValueError::Limit)?;
        self.charge(VALUE_BYTES)
    }
    pub fn blob(&mut self, length: usize, text: bool) -> Result<(), ValueError> {
        let bytes = length
            .checked_add(if text { TEXT_TERMINATOR_BYTES } else { 0 })
            .ok_or(ValueError::Limit)?;
        let words = bytes.checked_add(WORD_BYTES - 1).ok_or(ValueError::Limit)? / WORD_BYTES;
        self.charge(words.checked_mul(WORD_BYTES).ok_or(ValueError::Limit)?)
    }
    pub fn list(&mut self, length: usize, map: bool) -> Result<u32, ValueError> {
        let children = length
            .checked_mul(if map { MAP_CHILDREN } else { 1 })
            .ok_or(ValueError::Limit)?;
        if children > self.nodes {
            return Err(ValueError::Limit);
        }
        let entries = if map {
            length.checked_mul(ENTRY_BYTES).ok_or(ValueError::Limit)?
        } else {
            0
        };
        self.charge(WORD_BYTES.checked_add(entries).ok_or(ValueError::Limit)?)?;
        length.try_into().map_err(|_| ValueError::Limit)
    }
    fn charge(&mut self, bytes: usize) -> Result<(), ValueError> {
        self.bytes = self.bytes.checked_sub(bytes).ok_or(ValueError::Limit)?;
        Ok(())
    }
}
