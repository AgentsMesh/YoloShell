use super::*;
use yolo_process_test_support::ProcessIdentityObservation;

const SUCCESSOR_GENERATION: u64 = 2;
const FOREIGN_GENERATION: u64 = 3;

struct EvaluationFixture {
    _directory: TempDir,
    paths: Paths,
    expected: RuntimeAdoptionExpectation,
    candidate: RuntimeRecord,
    view: yolo_runtime::TerminalView,
}

impl EvaluationFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let paths = Paths::new(directory.path());
        let identity = ProcessIdentity::capture(std::process::id()).unwrap();
        let mut candidate = super::super::runtime_record_test_support::record_for_identity(
            &paths,
            Uuid::new_v4(),
            identity,
        );
        let bound =
            super::super::runtime_record_test_support::bind_record(&paths, candidate.clone());
        let expected = RuntimeAdoptionExpectation::new(bound);
        candidate.controller_generation = SUCCESSOR_GENERATION;
        let view = super::super::runtime_record_test_support::session_view(&candidate);
        Self {
            _directory: directory,
            paths,
            expected,
            candidate,
            view,
        }
    }

    fn publish(&self) {
        yolo_runtime::paths::atomic_write_json(
            &self.paths.runtime_record(self.candidate.terminal_id),
            &self.candidate,
        )
        .unwrap();
    }

    fn evaluate(&self) -> Result<AdoptionProgress, String> {
        self.publish();
        evaluate_runtime_with_observation(
            &self.expected,
            &self.candidate,
            Some(&self.view),
            SUCCESSOR_GENERATION,
            ProcessIdentityObservation::Matching,
        )
    }
}

#[test]
fn predecessor_generation_remains_pending() {
    let mut fixture = EvaluationFixture::new();
    fixture.candidate.controller_generation = fixture.expected.record().controller_generation;
    assert!(matches!(
        fixture.evaluate().unwrap(),
        AdoptionProgress::Pending(_)
    ));
}

#[test]
fn foreign_generation_fails_closed() {
    let mut fixture = EvaluationFixture::new();
    fixture.candidate.controller_generation = FOREIGN_GENERATION;
    assert!(fixture.evaluate().is_err());
}

#[test]
fn missing_runtime_view_fails_closed() {
    let fixture = EvaluationFixture::new();
    fixture.publish();
    assert!(
        evaluate_runtime_with_observation(
            &fixture.expected,
            &fixture.candidate,
            None,
            SUCCESSOR_GENERATION,
            ProcessIdentityObservation::Matching,
        )
        .is_err()
    );
}

#[test]
fn runtime_view_identity_drift_fails_closed() {
    let mut fixture = EvaluationFixture::new();
    fixture.view.runtime.as_mut().unwrap().worker_id = Uuid::new_v4();
    assert!(fixture.evaluate().is_err());
}

#[test]
fn unreachable_runtime_remains_pending() {
    let mut fixture = EvaluationFixture::new();
    fixture.view.runtime.as_mut().unwrap().status = TerminalRuntimeStatus::Unreachable;
    assert!(matches!(
        fixture.evaluate().unwrap(),
        AdoptionProgress::Pending(_)
    ));
}

#[test]
fn gone_or_unknown_worker_fails_closed() {
    let fixture = EvaluationFixture::new();
    fixture.publish();
    for observation in [
        ProcessIdentityObservation::Gone,
        ProcessIdentityObservation::Unknown,
    ] {
        assert!(
            evaluate_runtime_with_observation(
                &fixture.expected,
                &fixture.candidate,
                Some(&fixture.view),
                SUCCESSOR_GENERATION,
                observation,
            )
            .is_err()
        );
    }
}
