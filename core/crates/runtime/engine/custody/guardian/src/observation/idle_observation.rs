use super::{GuardianState, NamespaceLease, Paths, Progress};
use crate::process::MacosSecondaryCustody;
use anyhow::Result;

#[path = "idle_observation_kernel.rs"]
mod kernel;
use kernel::KernelObservation;

pub(super) trait ChangeObservation {
    fn unchanged(&self) -> Result<bool>;
    fn finish_validation(&self) -> Result<bool>;
}

pub(super) struct IdleObservation<W = KernelObservation> {
    observation: Option<W>,
}

impl<W> Default for IdleObservation<W> {
    fn default() -> Self {
        Self { observation: None }
    }
}

impl<W> IdleObservation<W> {
    pub(super) fn invalidate(&mut self) {
        self.observation = None;
    }
}

impl IdleObservation {
    pub(super) fn progress(
        &mut self,
        namespace: &NamespaceLease,
        paths: &Paths,
        custody: &MacosSecondaryCustody,
        state: &mut GuardianState,
    ) -> Result<Progress> {
        let runtime = state.runtime().clone();
        let holder = state.binding().host_holder();
        self.observe(
            eligible(state),
            || KernelObservation::capture(paths, &runtime, holder),
            || {
                let progress = super::progress(namespace, paths, custody, state)?;
                Ok((progress, eligible(state)))
            },
        )
    }
}

fn eligible(state: &GuardianState) -> bool {
    state.was_armed() && !state.needs_armed_promotion() && state.pending().is_none()
}

impl<W: ChangeObservation> IdleObservation<W> {
    fn observe(
        &mut self,
        eligible: bool,
        capture: impl FnOnce() -> Result<W>,
        validate: impl FnOnce() -> Result<(Progress, bool)>,
    ) -> Result<Progress> {
        if eligible
            && self
                .observation
                .as_ref()
                .is_some_and(|observation| observation.unchanged().unwrap_or(false))
        {
            return Ok(Progress::Pending);
        }
        self.invalidate();
        // Arm before full validation; anything changing during that validation invalidates it.
        let candidate = eligible.then(capture).and_then(Result::ok);
        let (progress, eligible) = validate()?;
        if eligible && progress == Progress::Pending {
            self.observation =
                candidate.filter(|observation| observation.finish_validation().unwrap_or(false));
        }
        Ok(progress)
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/idle_observation_tests.rs"]
mod tests;
