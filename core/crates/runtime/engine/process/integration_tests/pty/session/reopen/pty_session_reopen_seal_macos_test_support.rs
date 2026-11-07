use super::*;
use portable_pty::{PtySize, native_pty_system};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const TEST_CAPABILITY: &str = "pty-reopen-seal-test";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;
const PERMISSION_BITS: u32 = (libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO) as u32;

pub(super) fn install_seal(proof: &mut PtySessionProof) {
    let authority = proof.acquire_reopen_seal().unwrap();
    proof.install_reopen_seal(authority).unwrap();
}

pub(super) fn bound_proof(pair: &portable_pty::PtyPair) -> PtySessionProof {
    PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap()
}

pub(super) fn set_mode(path: &Path, value: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(value)).unwrap();
}

pub(super) fn mode(path: &Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & PERMISSION_BITS
}

pub(super) fn open_pty() -> portable_pty::PtyPair {
    native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap()
}
