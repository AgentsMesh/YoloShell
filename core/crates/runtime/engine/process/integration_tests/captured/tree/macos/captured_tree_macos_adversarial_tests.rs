use super::captured_tree_macos_support::LineageFixture;
use super::{CapturedProcessTree, macos_native};

#[test]
fn root_exit_and_setsid_cannot_hide_an_unobserved_slave_holder() {
    let mut fixture = LineageFixture::spawn();
    let root = fixture.root_identity();
    let root_kernel = macos_native::kernel_process_metadata(root.pid).unwrap();
    let mut tree = CapturedProcessTree::capture(root).unwrap();
    assert_eq!(tree.identities(), [root]);

    let leaf = fixture.release_and_observe_leaf();
    let leaf_kernel = macos_native::kernel_process_metadata(leaf.pid).unwrap();
    assert_eq!(leaf_kernel.process_group, leaf.pid as i32);
    assert_eq!(unsafe { libc::getsid(leaf.pid as i32) }, leaf.pid as i32);
    assert_ne!(leaf_kernel.parent_pid, root.pid);
    assert_eq!(leaf_kernel.parent_unique_id, root_kernel.unique_id);
    assert!(!tree.identities().contains(&leaf));

    let proof = tree.root_exit_proof_for_test(root);
    tree.mark_root_exited_unreaped(proof).unwrap();
    assert!(tree.refresh().unwrap().contains(&leaf));
    assert_eq!(tree.terminate_descendants(libc::SIGKILL).unwrap(), [leaf]);
    fixture.wait_for_leaf_exit();
    fixture.reap_root();
}
