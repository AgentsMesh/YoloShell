pub const FRAME_LENGTH_BYTES: usize = std::mem::size_of::<u32>();
// A terminal snapshot may contain eight MiB; leave bounded envelope headroom.
pub const MAX_FRAME_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const CLEAN_EOF_BYTES: usize = 0;
