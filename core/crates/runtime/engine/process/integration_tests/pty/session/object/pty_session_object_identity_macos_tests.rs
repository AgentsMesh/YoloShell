use super::*;

const FILESYSTEM_DEVICE: u32 = 71;
const ORIGINAL_INODE: u64 = 81;
const REPLACEMENT_INODE: u64 = 82;
const TERMINAL_RDEV: u32 = 91;

#[test]
fn inode_is_part_of_the_pty_object_identity() {
    let original = identity(ORIGINAL_INODE);
    let replacement = identity(REPLACEMENT_INODE);

    assert_ne!(original, replacement);
}

#[test]
fn vnode_identity_uses_all_three_stable_kernel_fields() {
    let mut stat = unsafe { std::mem::zeroed::<libc::vinfo_stat>() };
    stat.vst_dev = FILESYSTEM_DEVICE;
    stat.vst_ino = ORIGINAL_INODE;
    stat.vst_rdev = TERMINAL_RDEV;

    assert_eq!(
        MacosPtyObjectIdentity::from_vnode_stat(&stat),
        identity(ORIGINAL_INODE)
    );
}

fn identity(inode: u64) -> MacosPtyObjectIdentity {
    MacosPtyObjectIdentity::from_kernel_fields(FILESYSTEM_DEVICE, inode, TERMINAL_RDEV)
}
