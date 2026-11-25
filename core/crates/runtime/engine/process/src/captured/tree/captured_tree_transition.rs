use std::error::Error;
use std::fmt;

pub struct ProcessTransitionError<State> {
    state: State,
    source: anyhow::Error,
}

impl<State> ProcessTransitionError<State> {
    pub(super) fn new(state: State, source: anyhow::Error) -> Self {
        Self { state, source }
    }

    pub fn error(&self) -> &anyhow::Error {
        &self.source
    }

    pub fn into_state(self) -> State {
        self.state
    }

    pub fn into_parts(self) -> (State, anyhow::Error) {
        (self.state, self.source)
    }
}

impl<State> fmt::Debug for ProcessTransitionError<State> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ProcessTransitionError")
            .field(&self.source)
            .finish()
    }
}

impl<State> fmt::Display for ProcessTransitionError<State> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.source, formatter)
    }
}

impl<State> Error for ProcessTransitionError<State> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/captured_tree_transition_tests.rs"]
mod tests;
