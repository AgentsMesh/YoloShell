use super::{constants::*, support::*};
use crate::state::{Revocation, State};
use yolo_hub_agent_protocol::{
    AgentCapabilities, Capability, Freshness, ObservationCursor, ObservationSource, Reachability,
    RecoveryStatus,
};
use yolo_hub_connector_protocol::EnrollmentStatus;
use yolo_hub_connector_store::StoredState;
use yolo_hub_identity::{ConnectorInstanceId, ExecutionId};

#[test]
fn initial_and_published_states_validate_without_implied_recovery_or_native_source() {
    let (seed, state) = published();
    State::initial().validate(&seed).unwrap();
    state.validate(&seed).unwrap();
    let mut stale = state.clone();
    assert!(crate::observations::stale(&mut stale).unwrap());
    stale.validate(&seed).unwrap();
    assert!(!crate::observations::stale(&mut stale).unwrap());
    stale.last_sequence = 0;
    stale.last_publication = None;
    stale.context.as_mut().unwrap().instance = ConnectorInstanceId::new();
    stale.context.as_mut().unwrap().execution = ExecutionId::new();
    stale.validate(&seed).unwrap();
}

#[test]
fn no_current_publication_cannot_support_a_fresh_snapshot() {
    let (seed, mut state) = published();
    state.last_publication = None;
    state.last_sequence = 0;
    assert!(state.validate(&seed).is_err());
    crate::observations::stale(&mut state).unwrap();
    state.validate(&seed).unwrap();
}

#[test]
fn missing_context_and_nil_identities_cannot_manufacture_registry_state() {
    let mutations: &[fn(&mut State)] = &[
        |s| s.schema += SCHEMA_STEP,
        |s| s.revision = 0,
        |s| s.context = None,
        |s| s.context.as_mut().unwrap().stream = uuid::Uuid::nil(),
        |s| s.snapshot = None,
        |s| s.source_exited = true,
    ];
    for mutate in mutations {
        let (seed, mut state) = published();
        mutate(&mut state);
        assert!(state.validate(&seed).is_err());
    }
    let (seed, mut state) = published();
    state.last_publication = None;
    state.last_sequence = 0;
    state.snapshot = None;
    state.source_exited = true;
    assert!(state.validate(&seed).is_err());
}

#[test]
fn snapshot_cannot_gain_native_source_capability_recovery_or_unrelated_scope() {
    let mutations: &[fn(&mut yolo_hub_agent_protocol::AgentSnapshotFields)] = &[
        |f| f.source = ObservationSource::ProviderHook,
        |f| f.recovery = RecoveryStatus::Available,
        |f| {
            f.capabilities = AgentCapabilities {
                interrupt: Capability::Available {},
                submit_prompt: Capability::Unsupported {},
                respond_interaction: Capability::Unsupported {},
            }
        },
        |f| f.freshness = Freshness::Stale,
        |f| f.reachability = Reachability::Unreachable,
        |f| f.terminal.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new(),
        |f| {
            f.provider =
                yolo_hub_agent_protocol::ObservedProvider::new(OTHER_PROVIDER.into(), None, None)
                    .unwrap()
        },
    ];
    for mutate in mutations {
        let (seed, mut state) = published();
        snapshot(&mut state, mutate);
        assert!(state.validate(&seed).is_err());
    }
}

#[test]
fn revocation_requires_positive_prior_revision_exact_successor_and_non_nil_request() {
    for (prior, request, next) in [
        (0, uuid::Uuid::new_v4(), 1),
        (1, uuid::Uuid::nil(), 2),
        (1, uuid::Uuid::new_v4(), 3),
    ] {
        let (seed, mut state) = published();
        crate::observations::stale(&mut state).unwrap();
        state.status = EnrollmentStatus::Revoked;
        state.revision = next;
        state.revocation = Some(Revocation {
            request_id: request,
            expected_revision: prior,
        });
        assert!(state.validate(&seed).is_err());
    }
    let (seed, mut state) = published();
    crate::observations::stale(&mut state).unwrap();
    state.status = EnrollmentStatus::Revoked;
    assert!(state.validate(&seed).is_err());
    state.revision += STEP;
    state.revocation = Some(Revocation {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: state.revision - STEP,
    });
    state.validate(&seed).unwrap();
}

#[test]
fn current_and_stale_cursor_offsets_match_actual_publication_state() {
    let (seed, mut state) = published();
    snapshot(&mut state, |f| {
        f.cursor = ObservationCursor::new(
            f.cursor.target(),
            f.cursor.stream(),
            f.cursor.sequence() + STEP,
        )
        .unwrap()
    });
    assert!(state.validate(&seed).is_err());
    snapshot(&mut state, |f| {
        f.freshness = Freshness::Stale;
        f.reachability = Reachability::Unreachable;
    });
    state.validate(&seed).unwrap();
    snapshot(&mut state, |f| {
        f.cursor = ObservationCursor::new(
            f.cursor.target(),
            f.cursor.stream(),
            f.cursor.sequence() + STEP,
        )
        .unwrap()
    });
    assert!(state.validate(&seed).is_err());
    let (seed, mut state) = published();
    snapshot(&mut state, |f| {
        f.freshness = Freshness::Stale;
        f.reachability = Reachability::Unreachable;
    });
    assert!(state.validate(&seed).is_err());
}
