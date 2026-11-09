use super::constants::*;
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub(super) fn socket_digest(home: &Path, effective_user: u32) -> [u8; SOCKET_FULL_HASH_BYTES] {
    let mut hasher = Sha256::new();
    hasher.update(SOCKET_DIGEST_DOMAIN);
    hasher.update([SOCKET_DIGEST_SEPARATOR]);
    hasher.update(effective_user.to_be_bytes());
    hasher.update([SOCKET_DIGEST_SEPARATOR]);
    hasher.update(home.as_os_str().as_bytes());
    hasher.finalize().into()
}

pub(super) fn short_socket_hash(digest: &[u8; SOCKET_FULL_HASH_BYTES]) -> String {
    let mut encoded = String::with_capacity(SOCKET_HASH_HEX_LENGTH);
    for byte in digest.iter().take(SOCKET_HASH_BYTES) {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

pub(super) fn full_socket_hash(digest: &[u8; SOCKET_FULL_HASH_BYTES]) -> String {
    let mut encoded = String::with_capacity(SOCKET_FULL_HASH_HEX_LENGTH);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}
