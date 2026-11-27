use crate::model::{BootstrapDelivery, RuntimeRecord};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(in crate::worker) const AUDIT_PATH_ENV: &str = "TEST_RUNTIME_PERSISTENCE_AUDIT_PATH";
const AUDIT_READ_CONTEXT: &str = "read runtime persistence audit";
const AUDIT_WRITE_CONTEXT: &str = "write runtime persistence audit";

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::worker) enum RuntimePersistenceAuditEvent {
    Publication {
        bootstrap_delivery: BootstrapDelivery,
    },
    Removal {
        path: PathBuf,
    },
}

impl RuntimePersistenceAuditEvent {
    pub(in crate::worker) fn publishes(&self, delivery: BootstrapDelivery) -> bool {
        matches!(
            self,
            Self::Publication {
                bootstrap_delivery
            } if *bootstrap_delivery == delivery
        )
    }

    pub(in crate::worker) fn removes(&self, expected: &Path) -> bool {
        matches!(self, Self::Removal { path } if path == expected)
    }
}

pub(super) fn record_publication(runtime: &RuntimeRecord) -> Result<()> {
    record(RuntimePersistenceAuditEvent::Publication {
        bootstrap_delivery: runtime.bootstrap_delivery,
    })
}

pub(super) fn record_removal(path: &Path) -> Result<()> {
    record(RuntimePersistenceAuditEvent::Removal {
        path: path.to_owned(),
    })
}

pub(in crate::worker) fn read(path: &Path) -> Result<Vec<RuntimePersistenceAuditEvent>> {
    crate::paths::read_json(path).context(AUDIT_READ_CONTEXT)
}

fn record(event: RuntimePersistenceAuditEvent) -> Result<()> {
    let Some(path) = std::env::var_os(AUDIT_PATH_ENV).map(PathBuf::from) else {
        return Ok(());
    };
    record_at(&path, event)
}

fn record_at(path: &Path, event: RuntimePersistenceAuditEvent) -> Result<()> {
    let mut events = if path.exists() {
        read(path)?
    } else {
        Vec::new()
    };
    events.push(event);
    crate::paths::atomic_write_json(path, &events).context(AUDIT_WRITE_CONTEXT)
}

#[cfg(all(feature = "yolo-unit-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/runtime/runtime_persistence_test_hook_tests.rs"]
mod tests;
