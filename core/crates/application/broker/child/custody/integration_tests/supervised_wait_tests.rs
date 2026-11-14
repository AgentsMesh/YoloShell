use super::*;

#[path = "../test_support/supervised_wait.rs"]
mod support;
use support::Fixture;

#[test]
fn wait_accepts_readiness_enrichment_from_exact_active_custody() {
    let mut fixture = Fixture::new();
    fixture.paths.write_record(&fixture.ready).unwrap();
    let outcome = wait(
        &mut fixture.child.custody,
        &fixture.paths,
        &fixture.active,
        Instant::now() + yolo_remote_broker::BROKER_READY_TIMEOUT,
        |phase| phase == SupervisionPhase::Ready,
    )
    .unwrap();
    assert!(matches!(outcome, WaitOutcome::Reached));
    assert!(fixture.child.custody.try_wait().unwrap().is_none());
}

#[test]
fn wait_rejects_stripped_readiness_before_accepting_a_terminal_phase() {
    let mut fixture = Fixture::new();
    let mut stripped = fixture.active.clone();
    stripped.mark_settled().unwrap();
    fixture.paths.write_record(&stripped).unwrap();
    let result = wait(
        &mut fixture.child.custody,
        &fixture.paths,
        &fixture.ready,
        Instant::now() + yolo_remote_broker::BROKER_READY_TIMEOUT,
        |phase| phase == SupervisionPhase::Settled,
    );
    assert_eq!(result.err().unwrap().to_string(), RECORD_REPLACED);
    assert!(fixture.child.custody.try_wait().unwrap().is_none());
}

#[test]
fn wait_accepts_exact_readiness_through_settlement() {
    let mut fixture = Fixture::new();
    let mut settled = fixture.ready.clone();
    settled.mark_settled().unwrap();
    fixture.paths.write_record(&settled).unwrap();
    let outcome = wait(
        &mut fixture.child.custody,
        &fixture.paths,
        &fixture.ready,
        Instant::now() + yolo_remote_broker::BROKER_READY_TIMEOUT,
        |phase| phase == SupervisionPhase::Settled,
    )
    .unwrap();
    assert!(matches!(outcome, WaitOutcome::Reached));
}

#[test]
fn initialization_panic_after_spawn_terminates_and_reaps_the_owned_child() {
    const INITIALIZATION_FAILURE: &str = "injected supervised wait fixture initialization failure";
    const NO_WAITABLE_CHILD: libc::pid_t = -1;
    let mut identity = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Fixture::after_spawn(|child| {
            identity = child.identity();
            panic!("{INITIALIZATION_FAILURE}");
        })
    }));
    assert!(result.is_err());
    let identity = identity.unwrap();
    assert!(!identity.matches());
    let mut status = libc::c_int::default();
    assert_eq!(
        unsafe { libc::waitpid(identity.pid.try_into().unwrap(), &mut status, libc::WNOHANG) },
        NO_WAITABLE_CHILD
    );
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
