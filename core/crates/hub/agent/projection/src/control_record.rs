use crate::{
    constants::*,
    record::{Entry, Record},
};
use anyhow::{Result, ensure};
use yolo_hub_agent_protocol::{Capability, Freshness, Lifecycle, ObservationSource, Reachability};
use yolo_hub_identity::AgentTarget;

pub(crate) fn validate_entry(entry: &Entry) -> Result<()> {
    let fields = entry.snapshot.fields();
    let respondable = fields.capabilities.respond_interaction == Capability::Available {};
    if fields.interaction.is_some() || respondable {
        ensure!(
            fields.interaction.is_some()
                && respondable
                && fields.source == ObservationSource::ProviderHook
                && fields.freshness == Freshness::Current
                && fields.reachability == Reachability::Reachable
                && matches!(fields.lifecycle, Lifecycle::Running {})
                && fields.interaction.as_ref().is_some_and(|interaction| {
                    interaction.reference().target == fields.cursor.target()
                }),
            INVALID_RECORD
        );
    }
    let Some(control) = &entry.control_witness else {
        ensure!(
            fields.capabilities.interrupt == Capability::Unsupported {}
                && !respondable
                && fields.interaction.is_none(),
            INVALID_RECORD
        );
        return Ok(());
    };
    let witness = control.witness.fields();
    ensure!(
        control.cursor.target() == fields.cursor.target()
            && control.cursor.stream() == fields.cursor.stream()
            && control.cursor.sequence() <= fields.cursor.sequence()
            && witness.terminal_id == entry.subject.terminal_id.as_uuid()
            && witness.runtime_epoch == entry.runtime.as_uuid()
            && witness.kind.as_str() == entry.subject.kind
            && entry
                .process
                .is_some_and(|process| process.pid == witness.process_pid
                    && process.start_time == witness.process_start_time),
        INVALID_RECORD
    );
    if matches!(fields.capabilities.interrupt, Capability::Available {}) || respondable {
        ensure!(
            control.cursor == fields.cursor
                && fields.source == ObservationSource::ProviderHook
                && fields.freshness == Freshness::Current
                && fields.reachability == Reachability::Reachable
                && matches!(fields.lifecycle, Lifecycle::Running {}),
            INVALID_RECORD
        );
    }
    Ok(())
}

pub(crate) fn validate_retired(record: &Record) -> Result<()> {
    ensure!(
        record
            .entries
            .len()
            .saturating_add(record.retired_controls.len())
            <= MAX_AGENT_RECORDS,
        RECORD_LIMIT
    );
    for (execution, control) in &record.retired_controls {
        ensure!(
            control.cursor.target().execution_id == *execution
                && control.cursor.target().address.hub_id == record.hub
                && record
                    .entries
                    .get(&control.cursor.target().address.agent_id)
                    .is_some_and(|entry| entry.subject.terminal_id.as_uuid()
                        == control.witness.fields().terminal_id
                        && entry.subject.kind == control.witness.fields().kind.as_str())
                && record.entries.values().all(|entry| entry
                    .snapshot
                    .cursor()
                    .target()
                    .execution_id
                    != *execution),
            INVALID_RECORD
        );
    }
    Ok(())
}

pub(crate) fn retain_previous(
    record: &mut Record,
    previous: Option<&Entry>,
    next: AgentTarget,
) -> Result<()> {
    let Some(control) = previous
        .and_then(|entry| entry.control_witness.as_ref())
        .filter(|control| control.cursor.target() != next)
    else {
        return Ok(());
    };
    let key = control.cursor.target().execution_id;
    ensure!(
        record.retired_controls.contains_key(&key)
            || record
                .entries
                .len()
                .saturating_add(record.retired_controls.len())
                < MAX_AGENT_RECORDS,
        RECORD_LIMIT
    );
    if let Some(existing) = record.retired_controls.get(&key) {
        ensure!(existing == control, INVALID_RECORD);
    } else {
        record.retired_controls.insert(key, control.clone());
    }
    Ok(())
}
