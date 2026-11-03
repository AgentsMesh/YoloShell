use super::super::{FencedDescendants, ObservedProcess};
use super::wire::{ObservedProcessWire, ProcessSnapshotWireOutcome};
use crate::process::ProcessIdentity;
use yolo_agent_process::{ProcessSource, RuntimeProcessSource};

pub(super) fn stable(
    processes: Vec<ObservedProcess>,
    environment_keys: &[String],
) -> ProcessSnapshotWireOutcome {
    let processes = processes
        .into_iter()
        .map(ObservedProcessWire::from)
        .collect::<Vec<_>>();
    match super::validation::validate_processes(&processes, environment_keys) {
        Ok(()) => ProcessSnapshotWireOutcome::Stable { processes },
        Err(error) => ProcessSnapshotWireOutcome::Failed {
            detail: super::bounded_error(error.to_string()),
        },
    }
}

/// Observe the fenced descendants of `root` with the injected provider
/// environment keys. Detection selects adapters; the key set arrives from the
/// process composition root through the isolated scanner.
pub(super) fn observe(
    root: ProcessIdentity,
    environment_keys: &[String],
) -> ProcessSnapshotWireOutcome {
    match RuntimeProcessSource::from_owned(environment_keys.iter().cloned())
        .fenced_descendants(root)
    {
        Ok(FencedDescendants::Stable(processes)) => stable(processes, environment_keys),
        Ok(FencedDescendants::RootChanged) => ProcessSnapshotWireOutcome::RootChanged,
        Err(error) => ProcessSnapshotWireOutcome::Failed {
            detail: super::bounded_error(error.to_string()),
        },
    }
}
