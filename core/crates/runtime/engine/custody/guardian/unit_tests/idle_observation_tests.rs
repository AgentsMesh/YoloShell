use super::*;
use std::cell::Cell;
use std::rc::Rc;

const OBSERVATION_FAILURE: &str = "injected idle observation failure";
const UNEXPECTED_FULL_READ: &str = "unchanged observation repeated full validation";

#[derive(Clone, Default)]
struct Probe {
    dirty: Rc<Cell<bool>>,
    fail: Rc<Cell<bool>>,
}

impl ChangeObservation for Probe {
    fn unchanged(&self) -> Result<bool> {
        anyhow::ensure!(!self.fail.get(), OBSERVATION_FAILURE);
        Ok(!self.dirty.get())
    }

    fn finish_validation(&self) -> Result<bool> {
        self.unchanged()
    }
}

fn validate() -> Result<(Progress, bool)> {
    Ok((Progress::Pending, true))
}

fn armed(probe: &Probe) -> IdleObservation<Probe> {
    let mut idle = IdleObservation::default();
    idle.observe(true, || Ok(probe.clone()), validate).unwrap();
    assert!(idle.observation.is_some());
    idle
}

#[test]
fn unchanged_armed_observation_avoids_full_reads_and_rearming() {
    let mut idle = armed(&Probe::default());
    assert_eq!(
        idle.observe(
            true,
            || panic!("{UNEXPECTED_FULL_READ}"),
            || panic!("{UNEXPECTED_FULL_READ}"),
        )
        .unwrap(),
        Progress::Pending,
    );
}

#[test]
fn observer_is_registered_before_validation_and_changes_during_validation_are_not_cached() {
    let probe = Probe::default();
    let registered = Cell::new(false);
    let mut idle = IdleObservation::default();
    idle.observe(
        true,
        || {
            registered.set(true);
            Ok(probe.clone())
        },
        || {
            assert!(registered.get());
            probe.dirty.set(true);
            validate()
        },
    )
    .unwrap();
    assert!(idle.observation.is_none());
}

#[test]
fn every_dirty_or_failed_observation_runs_full_validation() {
    for fail in [false, true] {
        let probe = Probe::default();
        let mut idle = armed(&probe);
        probe.dirty.set(!fail);
        probe.fail.set(fail);
        let validated = Cell::new(false);
        idle.observe(
            true,
            || Ok(probe.clone()),
            || {
                validated.set(true);
                validate()
            },
        )
        .unwrap();
        assert!(validated.get());
        assert!(idle.observation.is_none());
    }
}

#[test]
fn unavailable_observer_preserves_full_validation_and_its_error() {
    let mut idle = IdleObservation::<Probe>::default();
    assert_eq!(
        idle.observe(true, || anyhow::bail!(OBSERVATION_FAILURE), validate,)
            .unwrap(),
        Progress::Pending
    );
    assert!(idle.observation.is_none());
    assert!(
        idle.observe(
            true,
            || Ok(Probe::default()),
            || anyhow::bail!(OBSERVATION_FAILURE),
        )
        .is_err()
    );
    assert!(idle.observation.is_none());
}

#[test]
fn pending_or_unpromoted_state_cannot_use_or_create_an_idle_observation() {
    let mut idle = armed(&Probe::default());
    let validated = Cell::new(false);
    idle.observe(
        false,
        || panic!("ineligible state attempted to arm"),
        || {
            validated.set(true);
            validate()
        },
    )
    .unwrap();
    assert!(validated.get());
    assert!(idle.observation.is_none());
}

#[test]
fn retirement_and_new_pending_state_discard_the_candidate() {
    for (progress, eligible) in [(Progress::Retired, true), (Progress::Pending, false)] {
        let mut idle = IdleObservation::default();
        idle.observe(true, || Ok(Probe::default()), || Ok((progress, eligible)))
            .unwrap();
        assert!(idle.observation.is_none());
    }
}

#[test]
fn successor_invalidation_forces_a_new_full_validation() {
    let mut idle = armed(&Probe::default());
    idle.invalidate();
    let validated = Cell::new(false);
    idle.observe(
        true,
        || Ok(Probe::default()),
        || {
            validated.set(true);
            validate()
        },
    )
    .unwrap();
    assert!(validated.get());
    assert!(idle.observation.is_some());
}
