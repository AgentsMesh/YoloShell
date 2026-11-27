use super::constants::*;
use crate::hash;
use std::io::{self, Cursor, Read};
use yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES;

#[test]
fn streaming_digest_matches_standard_vector_even_when_reads_fragment() {
    assert_eq!(hash::read(&mut Cursor::new(SOURCE), SIZE).unwrap(), DIGEST);
    assert_eq!(
        hash::read(&mut SingleByte(Cursor::new(SOURCE)), SIZE).unwrap(),
        DIGEST
    );
}

#[test]
fn short_or_growing_image_never_returns_a_partial_or_prefix_digest() {
    assert!(hash::read(&mut Cursor::new(SOURCE), SIZE + STEP).is_err());
    assert!(hash::read(&mut Cursor::new(SOURCE), SIZE - STEP).is_err());
}

#[test]
fn invalid_sizes_reject_before_io_and_read_failure_does_not_produce_an_image() {
    for invalid in [u64::default(), MAX_EXECUTABLE_IMAGE_BYTES + STEP] {
        assert!(hash::read(&mut NoRead, invalid).is_err());
    }
    assert!(hash::read(&mut FailedRead, SIZE).is_err());
}

struct SingleByte<R>(R);
impl<R: Read> Read for SingleByte<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let length = output.len().min(STEP as usize);
        self.0.read(&mut output[..length])
    }
}
struct NoRead;
impl Read for NoRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("invalid image size must be rejected before reading")
    }
}
struct FailedRead;
impl Read for FailedRead {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other(IO_FAILURE))
    }
}
