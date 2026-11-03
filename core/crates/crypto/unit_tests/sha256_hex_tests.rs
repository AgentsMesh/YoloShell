const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const MILLION_A_SHA256: &str = "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0";
const MILLION_BYTES: usize = 1_000_000;
const SHA256_BLOCK_BYTES: usize = 64;
const BYTE_STEP: usize = 1;
const BINARY_BYTES: usize = 65_536;
const CHUNK_LENGTHS: &[usize] = &[
    BYTE_STEP,
    SHA256_BLOCK_BYTES - BYTE_STEP,
    SHA256_BLOCK_BYTES,
    SHA256_BLOCK_BYTES + BYTE_STEP,
    BINARY_BYTES,
];

#[test]
fn sha256_hex_matches_standard_vectors() {
    assert_eq!(yolo_crypto::sha256_hex(b""), EMPTY_SHA256);
    assert_eq!(yolo_crypto::sha256_hex(b"abc"), ABC_SHA256);
}

#[test]
fn incremental_digest_matches_the_long_nist_vector() {
    let data = vec![b'a'; MILLION_BYTES];
    assert_eq!(yolo_crypto::sha256_hex(&data), MILLION_A_SHA256);
    let mut digest = yolo_crypto::Sha256Hasher::new();
    for chunk in data.chunks(SHA256_BLOCK_BYTES + BYTE_STEP) {
        digest.update(chunk);
    }
    assert_eq!(digest.finish_hex(), MILLION_A_SHA256);
}

#[test]
fn chunk_boundaries_and_empty_updates_preserve_the_binary_digest() {
    use sha2::{Digest, Sha256};
    let bytes: Vec<u8> = (u8::MIN..=u8::MAX)
        .cycle()
        .take(BINARY_BYTES + BYTE_STEP)
        .collect();
    let reference = format!("{:x}", Sha256::digest(&bytes));
    for &length in CHUNK_LENGTHS {
        let mut digest = yolo_crypto::Sha256Hasher::new();
        for chunk in bytes.chunks(length) {
            digest.update(&[]);
            digest.update(chunk);
        }
        assert_eq!(digest.finish_hex(), reference);
    }
    assert_eq!(yolo_crypto::Sha256Hasher::new().finish_hex(), EMPTY_SHA256);
}
