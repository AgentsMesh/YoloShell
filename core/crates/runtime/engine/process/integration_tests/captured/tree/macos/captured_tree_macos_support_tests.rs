use super::*;

#[test]
fn cleanup_handles_an_unreleased_root_without_a_leaf() {
    drop(LineageFixture::spawn());
}

#[test]
fn captured_leaf_keeps_the_root_alive_until_release() {
    let mut fixture = LineageFixture::spawn();
    let leaf = fixture.release_and_capture_leaf();

    assert!(fixture.root_live.matches());
    assert!(leaf.matches());
}

#[test]
fn root_exit_wait_observes_leaf_reparenting() {
    let mut fixture = LineageFixture::spawn();
    let leaf = fixture.release_and_capture_leaf();

    assert!(fixture.root_live.signal(libc::SIGKILL).unwrap());
    fixture.wait_for_root_exit_and_leaf_reparent();
    assert_root_exited_and_leaf_reparented(&fixture, leaf);
}

#[test]
fn observed_leaf_is_reparented_after_the_root_exits() {
    let mut fixture = LineageFixture::spawn();
    let leaf = fixture.release_and_observe_leaf();

    assert_root_exited_and_leaf_reparented(&fixture, leaf);
}

fn assert_root_exited_and_leaf_reparented(fixture: &LineageFixture, leaf: ProcessIdentity) {
    assert!(!fixture.root_live.matches());
    assert_ne!(
        crate::process::macos_native::kernel_process_metadata(leaf.pid)
            .unwrap()
            .parent_pid,
        fixture.root_identity.pid
    );
}

#[test]
fn cleanup_discovers_a_triggered_leaf_before_termination() {
    let fixture = LineageFixture::spawn();
    std::fs::write(&fixture.trigger, EMPTY_FILE).unwrap();
    drop(fixture);
}

#[test]
fn cleanup_bounds_discovery_after_the_root_exits() {
    let mut fixture = LineageFixture::spawn();
    fixture.root_live.signal(libc::SIGKILL).unwrap();
    fixture.reap_root();
    std::fs::write(&fixture.trigger, EMPTY_FILE).unwrap();
    drop(fixture);
}
