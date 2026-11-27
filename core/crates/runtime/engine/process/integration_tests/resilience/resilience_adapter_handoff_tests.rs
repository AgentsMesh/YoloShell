use super::captured_targets::install_test_signal_noop;
use super::resilience_adapter::install_test_kill_commit_handoff_panic;
use super::resilience_adapter_test_support::{AdapterTreeFixture, freeze, wait_gone};

const HANDOFF_PANIC_DETAIL: &str = "injected resilience kill commit handoff panic";

#[test]
fn panic_during_kill_commit_handoff_retains_committed_cleanup() {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let frozen = freeze(tree.begin_force());
    let noop = install_test_signal_noop(libc::SIGKILL, fixture.failure_target_index());
    let handoff = install_test_kill_commit_handoff_panic();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = frozen.kill();
    }));

    drop(handoff);
    drop(noop);
    let payload = outcome.unwrap_err();
    assert_eq!(
        super::resilience_cleanup_containment_test_support::panic_message(payload.as_ref()),
        Some(HANDOFF_PANIC_DETAIL)
    );
    fixture.reap_root();
    wait_gone(fixture.identities());
}
