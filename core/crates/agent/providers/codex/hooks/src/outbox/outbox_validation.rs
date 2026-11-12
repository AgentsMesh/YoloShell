use super::constants::{
    FIRST_SEQUENCE, MAX_ACKNOWLEDGED_EVENTS, MAX_PENDING_EVENTS, OUTBOX_SCHEMA_VERSION, errors,
};
use super::outbox::CodexOutboxState;
use crate::agent::hooks::{HookEnvelope, HookIdentity};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const EVENT_ID_DOMAIN: &str = "yolo.codex-hook.outbox-event-id.v1";
const EVENT_ID_HASH_BYTES: usize = 16;
const UUID_VARIANT_BYTE_INDEX: usize = 8;
const UUID_VARIANT_CLEAR_MASK: u8 = 0x3f;
const UUID_VARIANT_RFC_BITS: u8 = 0x80;
const UUID_VERSION_BYTE_INDEX: usize = 6;
const UUID_VERSION_CLEAR_MASK: u8 = 0x0f;
const UUID_VERSION_EIGHT_BITS: u8 = 0x80;

pub(super) fn validate(state: &CodexOutboxState) -> Result<()> {
    ensure!(
        state.schema_version == OUTBOX_SCHEMA_VERSION,
        errors::OUTBOX_CORRUPT
    );
    ensure!(
        state.pending.len() <= MAX_PENDING_EVENTS
            && state.acknowledged_events.len() <= MAX_ACKNOWLEDGED_EVENTS,
        errors::OUTBOX_CORRUPT
    );
    if let Some(owner) = &state.owner {
        owner.validate().map_err(|error| anyhow::anyhow!(error))?;
    }
    let mut ids = BTreeSet::new();
    validate_acknowledged(state, &mut ids)?;
    let mut expected = state
        .acknowledged_sequence
        .checked_add(FIRST_SEQUENCE)
        .context(errors::OUTBOX_CORRUPT)?;
    for event in &state.pending {
        ensure!(event.sequence.0 == expected, errors::OUTBOX_CORRUPT);
        ensure!(ids.insert(event.event_id), errors::OUTBOX_CORRUPT);
        expected = expected
            .checked_add(FIRST_SEQUENCE)
            .context(errors::OUTBOX_CORRUPT)?;
    }
    ensure!(state.next_sequence == expected, errors::OUTBOX_CORRUPT);
    validate_event_integrity(state)?;
    validate_owner_events(state.owner.as_ref(), state)?;
    super::outbox_occurrence_state::validate(state)?;
    super::outbox_rejection::validate(state)?;
    Ok(())
}

fn validate_event_integrity(state: &CodexOutboxState) -> Result<()> {
    for event in state.acknowledged_events.iter().chain(state.pending.iter()) {
        ensure!(!event.event_id.is_nil(), errors::OUTBOX_CORRUPT);
        match state.durable_rejection_for(event)? {
            Some(marker) => {
                marker.validate_for(event)?;
                super::outbox_envelope_validation::validate_rejected(event, marker.violation())?
            }
            None => super::outbox_envelope_validation::validate(event)?,
        }
    }
    Ok(())
}

pub(super) fn derive_allocated_event_id(event: &HookEnvelope) -> Result<uuid::Uuid> {
    ensure!(event.event_id.is_nil(), errors::OUTBOX_CORRUPT);
    let source = (EVENT_ID_DOMAIN, &event.identity, event.sequence, event);
    let encoded = serde_json::to_vec(&source).context(errors::OUTBOX_CORRUPT)?;
    let digest = Sha256::digest(encoded);
    let mut bytes = [0_u8; EVENT_ID_HASH_BYTES];
    bytes.copy_from_slice(&digest[..EVENT_ID_HASH_BYTES]);
    bytes[UUID_VERSION_BYTE_INDEX] =
        (bytes[UUID_VERSION_BYTE_INDEX] & UUID_VERSION_CLEAR_MASK) | UUID_VERSION_EIGHT_BITS;
    bytes[UUID_VARIANT_BYTE_INDEX] =
        (bytes[UUID_VARIANT_BYTE_INDEX] & UUID_VARIANT_CLEAR_MASK) | UUID_VARIANT_RFC_BITS;
    Ok(uuid::Uuid::from_bytes(bytes))
}

pub(super) fn find_explicit_replay<'a>(
    state: &CodexOutboxState,
    incoming: &HookEnvelope,
    identity: Option<&super::CodexOccurrenceIdentity>,
) -> Result<Option<HookEnvelope>> {
    super::outbox_occurrence_state::find_replay(state, incoming, identity)
}

pub(super) fn find_acknowledged(
    state: &CodexOutboxState,
    sequence: u64,
    event_id: uuid::Uuid,
) -> Result<bool> {
    let by_sequence = state
        .acknowledged_events
        .iter()
        .find(|event| event.sequence.0 == sequence);
    if let Some(event) = by_sequence {
        ensure!(event.event_id == event_id, errors::ACK_REPLAY_UNKNOWN);
    }
    Ok(by_sequence.is_some())
}

fn validate_acknowledged(state: &CodexOutboxState, ids: &mut BTreeSet<uuid::Uuid>) -> Result<()> {
    let retained_limit = u64::try_from(MAX_ACKNOWLEDGED_EVENTS).context(errors::OUTBOX_CORRUPT)?;
    let expected_len = state.acknowledged_sequence.min(retained_limit);
    ensure!(
        u64::try_from(state.acknowledged_events.len()).context(errors::OUTBOX_CORRUPT)?
            == expected_len,
        errors::OUTBOX_CORRUPT
    );
    let first_sequence = state
        .acknowledged_sequence
        .checked_sub(expected_len)
        .and_then(|prefix| prefix.checked_add(FIRST_SEQUENCE))
        .context(errors::OUTBOX_CORRUPT)?;
    for (offset, event) in state.acknowledged_events.iter().enumerate() {
        let expected = first_sequence
            .checked_add(u64::try_from(offset).context(errors::OUTBOX_CORRUPT)?)
            .context(errors::OUTBOX_CORRUPT)?;
        ensure!(event.sequence.0 == expected, errors::OUTBOX_CORRUPT);
        ensure!(ids.insert(event.event_id), errors::OUTBOX_CORRUPT);
    }
    Ok(())
}

fn validate_owner_events(owner: Option<&HookIdentity>, state: &CodexOutboxState) -> Result<()> {
    let Some(owner) = owner else {
        return Ok(());
    };
    for event in state.acknowledged_events.iter().chain(state.pending.iter()) {
        ensure!(event.identity == *owner, errors::OUTBOX_OWNER);
    }
    Ok(())
}

pub(super) fn bind(
    state: &mut CodexOutboxState,
    configured: Option<&HookIdentity>,
    incoming: &HookIdentity,
) -> Result<()> {
    let requested = configured.unwrap_or(incoming);
    ensure!(incoming == requested, errors::OUTBOX_OWNER);
    if let Some(owner) = &state.owner {
        ensure!(owner == requested, errors::OUTBOX_OWNER);
    } else {
        let all_match = state
            .acknowledged_events
            .iter()
            .chain(state.pending.iter())
            .all(|event| event.identity == *requested);
        ensure!(all_match, errors::OUTBOX_OWNER_MISSING);
        state.owner = Some(requested.clone());
    }
    Ok(())
}

pub(super) fn check_configured(
    state: &CodexOutboxState,
    configured: Option<&HookIdentity>,
) -> Result<()> {
    if let Some(owner) = configured {
        if let Some(existing) = &state.owner {
            ensure!(existing == owner, errors::OUTBOX_OWNER);
        }
        validate_owner_events(Some(owner), state)?;
    }
    Ok(())
}

pub(super) fn same_event(left: &HookEnvelope, right: &HookEnvelope) -> bool {
    left.schema_version == right.schema_version
        && left.event_id == right.event_id
        && left.identity == right.identity
        && left.provider == right.provider
        && left.provider_version == right.provider_version
        && left.provider_session_id == right.provider_session_id
        && left.turn_id == right.turn_id
        && left.agent_id == right.agent_id
        && left.tool_name == right.tool_name
        && left.tool_use_id == right.tool_use_id
        && left.stop_hook_active == right.stop_hook_active
        && left.metadata == right.metadata
        && left.event == right.event
}

pub(super) fn trim_acknowledged(state: &mut CodexOutboxState) {
    while state.acknowledged_events.len() > MAX_ACKNOWLEDGED_EVENTS {
        state.acknowledged_events.pop_front();
    }
    super::outbox_rejection::trim(state);
}
