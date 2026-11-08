use capnp::message::{Allocator, HeapAllocator};
use std::ptr::NonNull;
use zeroize::Zeroize;

pub(crate) struct Scrubbed(HeapAllocator);
impl Default for Scrubbed {
    fn default() -> Self {
        Self(HeapAllocator::new())
    }
}

// The underlying allocator provides zeroed, aligned, non-overlapping segments.
unsafe impl Allocator for Scrubbed {
    fn allocate_segment(&mut self, minimum_size: u32) -> (NonNull<u8>, u32) {
        self.0.allocate_segment(minimum_size)
    }
    unsafe fn deallocate_segment(&mut self, ptr: NonNull<u8>, word_size: u32, words_used: u32) {
        // Caller guarantees a live segment and exact size (Allocator contract).
        let bytes = word_size as usize * std::mem::size_of::<capnp::Word>();
        unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), bytes) }.zeroize();
        if word_size != 0 {
            unsafe {
                self.0.deallocate_segment(ptr, word_size, words_used);
            }
        }
    }
}
