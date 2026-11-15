#[path = "../../../test_support/unit/codex_hook/control/control_source_fixture.rs"]
mod support;
use std::{cell::Cell, sync::mpsc, time::Duration};
use support::ControlSource;
use yolo_agent_control_contract::authority_digest;

const CALLBACK_TIMEOUT: Duration = Duration::from_secs(5);

#[path = "control_candidate_tests.rs"]
mod candidate_tests;

#[test]
fn current_source_capture_repeats_and_reentrant_control_is_fenced() {
    let source = ControlSource::new();
    let first = source
        .run(|lineage, verify| {
            verify()?;
            Ok(authority_digest(lineage)?)
        })
        .unwrap();
    let second = source
        .run(|lineage, verify| {
            verify()?;
            assert!(source.run(|_, _| Ok(())).is_err());
            Ok(authority_digest(lineage)?)
        })
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        source
            .run(|lineage, _| Ok(authority_digest(lineage)?))
            .unwrap(),
        first
    );
}

#[test]
fn advanced_live_reducer_rejects_unchanged_persisted_projection_before_effect() {
    let source = ControlSource::new();
    let before = source.authority();
    source.projection.stop();
    assert_eq!(source.authority(), before);
    let called = Cell::new(false);
    assert!(
        source
            .run(|_, _| {
                called.set(true);
                Ok(())
            })
            .is_err()
    );
    assert!(!called.get());
    source.persist_current();
    source
        .run(|_, _| {
            called.set(true);
            Ok(())
        })
        .unwrap();
    assert!(called.get());
}

#[derive(Clone, Copy)]
enum InvalidSource {
    RetiredRoute,
    PoisonedLifecycle,
    RetiredRegistration,
    ReplacedSnapshot,
}

#[test]
fn retired_route_registration_and_invalid_source_do_not_enter_callback() {
    for invalid in [
        InvalidSource::RetiredRoute,
        InvalidSource::PoisonedLifecycle,
        InvalidSource::RetiredRegistration,
        InvalidSource::ReplacedSnapshot,
    ] {
        let source = ControlSource::new();
        let entry = &source.projection.committed.as_ref().unwrap().entry;
        let runtime = &source.projection.authority.as_ref().unwrap().runtime;
        match invalid {
            InvalidSource::RetiredRoute => {
                crate::worker::codex_hook::router::test_support::support::ready_retirement(
                    source.projection.router.begin_retirement(entry),
                )
                .commit()
                .unwrap()
            }
            InvalidSource::PoisonedLifecycle => runtime.poison_lifecycle_for_test(),
            InvalidSource::RetiredRegistration => runtime.retire_control_registration_for_test(),
            InvalidSource::ReplacedSnapshot => {
                crate::codex_hooks::replace_snapshot_same_content(&runtime.snapshot_path())
            }
        }
        let called = Cell::new(false);
        assert!(
            source
                .run(|_, _| {
                    called.set(true);
                    Ok(())
                })
                .is_err()
        );
        assert!(!called.get());
    }
}

#[test]
fn source_lifecycle_cannot_advance_until_effect_callback_releases_its_guard() {
    let source = ControlSource::new();
    let before = source.authority();
    std::thread::scope(|scope| {
        let (entered, observed) = mpsc::channel();
        let (done, completed) = mpsc::channel();
        source
            .run(|_, _| {
                let projection = &source.projection;
                scope.spawn(move || {
                    entered.send(()).unwrap();
                    projection.stop();
                    done.send(()).unwrap();
                });
                observed.recv_timeout(CALLBACK_TIMEOUT).unwrap();
                assert!(matches!(
                    completed.try_recv(),
                    Err(mpsc::TryRecvError::Empty)
                ));
                Ok(())
            })
            .unwrap();
        completed.recv_timeout(CALLBACK_TIMEOUT).unwrap();
    });
    assert_eq!(source.authority(), before);
    assert!(source.run(|_, _| Ok(())).is_err());
}

#[test]
fn post_intent_codex_verifier_detects_replaced_snapshot_using_retained_publication_proof() {
    let source = ControlSource::new();
    let runtime = &source.projection.authority.as_ref().unwrap().runtime;
    let called = Cell::new(false);
    assert!(
        source
            .run(|_, verify| {
                verify()?;
                crate::codex_hooks::replace_snapshot_same_content(&runtime.snapshot_path());
                verify()?;
                called.set(true);
                Ok(())
            })
            .is_err()
    );
    assert!(!called.get());
}
