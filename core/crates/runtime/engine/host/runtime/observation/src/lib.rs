//! One Host runtime observation: the runtime view a caller may render and,
//! when the view was authenticated, the exact record it was authenticated
//! against.

use yolo_terminal_model::{RuntimeRecord, RuntimeView};

pub struct RuntimeObservation {
    pub runtime: RuntimeView,
    pub authenticated_record: Option<RuntimeRecord>,
}

impl RuntimeObservation {
    pub fn authenticated(runtime: RuntimeView, record: &RuntimeRecord) -> Self {
        Self {
            runtime,
            authenticated_record: Some(record.clone()),
        }
    }

    pub fn unauthenticated(runtime: RuntimeView) -> Self {
        Self {
            runtime,
            authenticated_record: None,
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated_record.is_some()
    }

    pub fn authenticates(&self, record: &RuntimeRecord) -> bool {
        self.authenticated_record
            .as_ref()
            .is_some_and(|authenticated| record.has_same_process_identity(authenticated))
    }
}
