use super::resilience_adapter_test_support::{
    AdapterTreeFixture, freeze, wait_running, wait_stopped,
};
use super::resilience_cleanup_containment_test_support::panic_message;
use super::{install_test_live_signal_failure, install_test_live_signal_observer};
use std::cell::Cell;
use std::rc::Rc;

const OUTER_PANIC: &str = "injected persistent cleanup outer unwind";
const RESUME_FAILURE: &str = "injected persistent retained resume failure";
const KILL_ESCALATION_DETAIL: &str =
    "persistent retained resume failure escalated cancellation to destruction";

#[test]
fn persistent_resume_failure_never_escalates_cancellation_to_kill() {
    let (fixture, tree) = AdapterTreeFixture::spawn();
    let frozen = freeze(tree.begin_force());
    let current_signal = Rc::new(Cell::new(libc::c_int::default()));
    let observed_signal = Rc::clone(&current_signal);
    let saw_kill = Rc::new(Cell::new(false));
    let observed_kill = Rc::clone(&saw_kill);
    let observer = install_test_live_signal_observer(move |signal| {
        observed_signal.set(signal);
        if signal == libc::SIGKILL {
            observed_kill.set(true);
        }
    });
    let failed_signal = Rc::clone(&current_signal);
    let failure = install_test_live_signal_failure(move || {
        (failed_signal.get() == libc::SIGCONT).then(|| RESUME_FAILURE.to_owned())
    });

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _frozen = frozen;
        panic!("{OUTER_PANIC}");
    }));

    drop(failure);
    drop(observer);
    assert_eq!(
        panic_message(outcome.unwrap_err().as_ref()),
        Some(OUTER_PANIC)
    );
    assert!(!saw_kill.get(), "{KILL_ESCALATION_DETAIL}");
    wait_stopped(fixture.identities());
    for identity in fixture.identities() {
        assert!(identity.signal(libc::SIGCONT).unwrap());
    }
    wait_running(fixture.identities());
}
