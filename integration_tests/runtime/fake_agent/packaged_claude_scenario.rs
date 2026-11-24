use crate::fake_agent_constants::{PACKAGED_E2E_ENABLED, PACKAGED_E2E_ENV};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Scenario {
    Full,
    StartOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Step {
    SessionStart,
    TranscriptRunning,
    UserPrompt,
    TranscriptWaiting,
    Stop,
    WaitingNotification,
}

const FULL_STEPS: &[Step] = &[
    Step::SessionStart,
    Step::TranscriptRunning,
    Step::UserPrompt,
    Step::TranscriptWaiting,
    Step::Stop,
    Step::WaitingNotification,
];
const START_ONLY_STEPS: &[Step] = &[Step::SessionStart];

impl Scenario {
    pub(super) const fn steps(self) -> &'static [Step] {
        match self {
            Self::Full => FULL_STEPS,
            Self::StartOnly => START_ONLY_STEPS,
        }
    }
}

pub(super) fn configured_scenario(packaged: Option<&str>) -> Scenario {
    if packaged == Some(PACKAGED_E2E_ENABLED) {
        return Scenario::Full;
    }
    Scenario::StartOnly
}

pub(super) fn environment_name() -> &'static str {
    PACKAGED_E2E_ENV
}
