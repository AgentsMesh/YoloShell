pub const SCHEMA_VERSION: u16 = 1;
// A map step traverses its list, entry struct and child value pointer.
const POINTERS_PER_VALUE_DEPTH: usize = 3;
const RPC_ENVELOPE_POINTER_RESERVE: usize = 8;
pub const CAPNP_NESTING_LIMIT: i32 =
    (MAX_VALUE_DEPTH * POINTERS_PER_VALUE_DEPTH + RPC_ENVELOPE_POINTER_RESERVE) as i32;
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_VALUE_DEPTH: usize = 64;
pub const MAX_VALUE_NODES: usize = 256 * 1024;
pub const WORD_BYTES: usize = 8;
pub const CHILD_DEPTH: usize = 1;
