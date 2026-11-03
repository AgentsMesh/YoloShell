use super::captured_targets::install_test_signal_noop;
use super::resilience_adapter_test_support::{
    AdapterTreeFixture, freeze, wait_gone, wait_running, wait_stopped,
};
use super::resilience_cleanup_containment_test_support::{
    adversarial_error, panic_message, panic_subscriber, panic_with_payload,
};
use super::{
    ResilienceKillCommitPoll, install_test_live_execution_state_failure,
    install_test_live_signal_failure,
};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::Ordering;

const OUTER_PANIC: &str = "injected resilience wrapper outer unwind";
const SIGNAL_ERROR: &str = "injected resilience cleanup signal error";
const SIGNAL_PANIC: &str = "injected resilience cleanup signal panic";
const PENDING_REQUIRED: &str = "resilience kill did not remain pending";
const FIRST_INJECTION: usize = 1;

#[test]
fn freezing_drop_resumes_after_signal_error_and_trace_panics_during_outer_unwind() {
    let (fixture, tree) = AdapterTreeFixture::spawn();
    let failed = fixture.failure_target();
    let survivors = fixture
        .identities()
        .iter()
        .copied()
        .filter(|identity| *identity != failed)
        .collect::<Vec<_>>();
    assert!(failed.signal(libc::SIGSTOP).unwrap());
    wait_stopped(&[failed]);
    let freezing = tree.begin_force();
    let injection = Rc::new(Cell::new(FIRST_INJECTION));
    let remaining = Rc::clone(&injection);
    let failure = install_test_live_signal_failure(move || {
        (remaining.replace(usize::default()) == FIRST_INJECTION).then(|| SIGNAL_ERROR.to_owned())
    });
    let (subscriber, events, payload_drops) = panic_subscriber();
    let _tracing_test = yolo_application_test_fixtures::tracing::serialize_subscriber_tests();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        tracing::subscriber::with_default(subscriber, || {
            let _freezing = freezing;
            panic!("{OUTER_PANIC}");
        });
    }));

    drop(failure);
    assert_outer_panic(outcome);
    assert_eq!(events.load(Ordering::SeqCst), 1);
    assert_eq!(payload_drops.load(Ordering::SeqCst), 1);
    wait_running(&survivors);
    wait_running(&[failed]);
}

#[test]
fn frozen_drop_contains_signal_panic_and_resumes_targets_during_outer_unwind() {
    let (fixture, tree) = AdapterTreeFixture::spawn();
    let frozen = freeze(tree.begin_force());
    let failure = panic_once_on_live_signal();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _frozen = frozen;
        panic!("{OUTER_PANIC}");
    }));

    drop(failure);
    assert_outer_panic(outcome);
    wait_running(fixture.identities());
}

#[test]
fn frozen_kill_drop_contains_observation_error_format_and_drop_panics() {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let mut frozen = freeze(tree.begin_force());
    frozen.arm_kill_cleanup();
    let displays = std::sync::Arc::new(std::sync::atomic::AtomicUsize::default());
    let drops = std::sync::Arc::new(std::sync::atomic::AtomicUsize::default());
    let remaining = Rc::new(Cell::new(FIRST_INJECTION));
    let injected = Rc::clone(&remaining);
    let failure = install_test_live_execution_state_failure({
        let displays = std::sync::Arc::clone(&displays);
        let drops = std::sync::Arc::clone(&drops);
        move || {
            (injected.replace(usize::default()) == FIRST_INJECTION)
                .then(|| adversarial_error(&displays, &drops))
        }
    });

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drop(frozen)));

    drop(failure);
    assert!(outcome.is_ok());
    assert_eq!(displays.load(Ordering::SeqCst), FIRST_INJECTION);
    assert_eq!(drops.load(Ordering::SeqCst), FIRST_INJECTION);
    fixture.reap_root();
    wait_gone(fixture.identities());
}

#[test]
fn frozen_kill_drop_contains_observation_panic_payload_during_outer_unwind() {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let mut frozen = freeze(tree.begin_force());
    frozen.arm_kill_cleanup();
    let payload_drops = std::sync::Arc::new(std::sync::atomic::AtomicUsize::default());
    let remaining = Rc::new(Cell::new(FIRST_INJECTION));
    let injected = Rc::clone(&remaining);
    let observed_drops = std::sync::Arc::clone(&payload_drops);
    let failure = install_test_live_execution_state_failure(move || {
        if injected.replace(usize::default()) == FIRST_INJECTION {
            panic_with_payload(&observed_drops);
        }
        None
    });

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _frozen = frozen;
        panic!("{OUTER_PANIC}");
    }));

    drop(failure);
    assert_outer_panic(outcome);
    assert_eq!(payload_drops.load(Ordering::SeqCst), FIRST_INJECTION);
    fixture.reap_root();
    wait_gone(fixture.identities());
}

#[test]
fn kill_pending_drop_contains_signal_panic_during_outer_unwind() {
    let (mut fixture, tree) = AdapterTreeFixture::spawn();
    let mut frozen = freeze(tree.begin_force());
    frozen.downgrade_kill_basis_to_ordinary_for_test();
    let noop = install_test_signal_noop(libc::SIGKILL, fixture.failure_target_index());
    let pending = match frozen.kill().unwrap() {
        ResilienceKillCommitPoll::Pending(pending) => pending,
        ResilienceKillCommitPoll::Refreeze(_) => panic!("{PENDING_REQUIRED}"),
    };
    drop(noop);
    wait_stopped(&[fixture.failure_target()]);
    let failure = panic_once_on_live_signal();

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _pending = pending;
        panic!("{OUTER_PANIC}");
    }));

    drop(failure);
    assert_outer_panic(outcome);
    fixture.reap_root();
    wait_gone(fixture.identities());
}

fn panic_once_on_live_signal() -> impl Drop {
    let injection = Rc::new(Cell::new(FIRST_INJECTION));
    install_test_live_signal_failure(move || {
        if injection.replace(usize::default()) == FIRST_INJECTION {
            panic!("{SIGNAL_PANIC}");
        }
        None
    })
}

fn assert_outer_panic(outcome: std::thread::Result<()>) {
    assert_eq!(
        panic_message(outcome.unwrap_err().as_ref()),
        Some(OUTER_PANIC)
    );
}
