use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct ResilienceTransitionError<State> {
    state: State,
    source: anyhow::Error,
}

impl<State> ResilienceTransitionError<State> {
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

impl<State> Debug for ResilienceTransitionError<State> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("ResilienceTransitionError")
            .field(&self.source)
            .finish()
    }
}

impl<State> Display for ResilienceTransitionError<State> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.source, formatter)
    }
}

impl<State> Error for ResilienceTransitionError<State> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
