use super::*;
use std::os::fd::AsRawFd;

#[test]
fn native_descriptor_inventory_contains_the_open_executable() {
    let executable = std::fs::File::open(std::env::current_exe().unwrap()).unwrap();
    let descriptors = list_descriptors(
        std::process::id(),
        crate::process::constants::MAX_PTY_PROOF_FILE_DESCRIPTORS,
    )
    .unwrap();
    let observed = descriptors
        .iter()
        .find(|entry| entry.proc_fd == executable.as_raw_fd())
        .unwrap();
    assert_eq!(observed.proc_fdtype, libc::PROX_FDTYPE_VNODE as u32);
}
