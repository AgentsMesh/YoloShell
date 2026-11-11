use super::TerminalRuntimeStatus;

impl TerminalRuntimeStatus {
    pub fn accepts_exact_termination_observation(self, observed: Self) -> bool {
        self == observed
            || matches!(
                (self, observed),
                (
                    Self::Starting
                        | Self::Running
                        | Self::Finalizing
                        | Self::OutputBlocked
                        | Self::Lost
                        | Self::Unreachable,
                    Self::Exited
                )
            )
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/terminal_runtime_status_inline_tests.rs"]
mod tests;
