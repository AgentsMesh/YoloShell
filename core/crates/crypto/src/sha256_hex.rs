use ring::digest::{Context, SHA256};
use std::fmt::Write;

const HEX_DIGITS_PER_BYTE: usize = 2;
const SHA256_BYTES: usize = 32;
const SHA256_HEX_LENGTH: usize = SHA256_BYTES * HEX_DIGITS_PER_BYTE;
const STRING_WRITE_FAILURE: &str = "writing SHA-256 to a String cannot fail";

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut digest = Sha256Hasher::new();
    digest.update(bytes);
    digest.finish_hex()
}

/// Incremental SHA-256 using the library's CPU-dispatched implementation.
/// Callers retain responsibility for complete reads and file identity checks.
pub struct Sha256Hasher(Context);

impl Sha256Hasher {
    pub fn new() -> Self {
        Self(Context::new(&SHA256))
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    pub fn finish_hex(self) -> String {
        let digest = self.0.finish();
        let mut encoded = String::with_capacity(SHA256_HEX_LENGTH);
        for byte in digest.as_ref() {
            write!(&mut encoded, "{byte:02x}").expect(STRING_WRITE_FAILURE);
        }
        encoded
    }
}
