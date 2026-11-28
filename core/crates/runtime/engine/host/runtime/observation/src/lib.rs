//! One Host runtime observation: the runtime view a caller may render and,
//! when the view was authenticated, the exact record it was authenticated
//! against.

use yolo_terminal_model::{RuntimeRecord, RuntimeView};

pub struct RuntimeObservation {
    pub runtime: RuntimeView,
    /// The RuntimeRecord used to build this observation. It may be present
    /// without authentication when the Worker could not answer.
    pub observed_record: Option<RuntimeRecord>,
    authenticated: bool,
}

impl RuntimeObservation {
    pub fn authenticated(runtime: RuntimeView, record: &RuntimeRecord) -> Self {
        Self {
            runtime,
            observed_record: Some(record.clone()),
            authenticated: true,
        }
    }

    pub fn unauthenticated(runtime: RuntimeView) -> Self {
        Self {
            runtime,
            observed_record: None,
            authenticated: false,
        }
    }

    pub fn unauthenticated_with_record(runtime: RuntimeView, record: &RuntimeRecord) -> Self {
        Self {
            runtime,
            observed_record: Some(record.clone()),
            authenticated: false,
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }

    pub fn authenticates(&self, record: &RuntimeRecord) -> bool {
        self.authenticated
            && self
                .observed_record
                .as_ref()
                .is_some_and(|observed| record.has_same_process_identity(observed))
    }
}
