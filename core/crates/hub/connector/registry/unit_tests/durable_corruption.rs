use super::{durable_corruption_support::*, support::snapshot};
use crate::state::{Revocation, State};
use yolo_hub_agent_protocol::{
    Activity, Interaction, InteractionKind, InteractionReference, Lifecycle, ObservationCursor,
};
use yolo_hub_identity::{AgentId, ExecutionId};

#[test]
fn restart_rejects_observation_address_execution_or_stream_detached_from_its_publication() {
    let changes: &[fn(&mut State)] = &[
        |state| {
            snapshot(state, |fields| {
                let mut target = fields.cursor.target();
                target.address.agent_id = AgentId::new();
                fields.cursor = ObservationCursor::new(
                    target,
                    fields.cursor.stream(),
                    fields.cursor.sequence(),
                )
                .unwrap();
            })
        },
        |state| {
            snapshot(state, |fields| {
                let mut target = fields.cursor.target();
                target.execution_id = ExecutionId::new();
                fields.cursor = ObservationCursor::new(
                    target,
                    fields.cursor.stream(),
                    fields.cursor.sequence(),
                )
                .unwrap();
            })
        },
        |state| {
            snapshot(state, |fields| {
                fields.cursor = ObservationCursor::new(
                    fields.cursor.target(),
                    uuid::Uuid::new_v4(),
                    fields.cursor.sequence(),
                )
                .unwrap();
            })
        },
        |state| {
            state
                .last_publication
                .as_mut()
                .unwrap()
                .agent
                .address
                .agent_id = AgentId::new()
        },
    ];
    for change in changes {
        rejects_persisted_change(change);
    }
}

#[test]
fn restart_rejects_a_snapshot_that_changes_the_committed_lifecycle_or_activity() {
    rejects_persisted_change(|state| {
        snapshot(state, |fields| fields.lifecycle = Lifecycle::Unknown {})
    });
    rejects_persisted_change(|state| snapshot(state, |fields| fields.activity = Activity::Idle));
}

#[test]
fn restart_cannot_adopt_publication_or_snapshot_left_without_activation_context() {
    for preserve_publication in [true, false] {
        rejects_persisted_change(|state| {
            state.context = None;
            state.last_sequence = super::super::constants::INITIAL_SEQUENCE;
            if !preserve_publication {
                state.last_publication = None;
            }
        });
    }
    rejects_persisted_change(|state| state.snapshot = None);
}

#[test]
fn retained_exit_tombstone_cannot_be_moved_to_another_execution_or_erased() {
    rejects_persisted_change(|state| {
        mark_exited(state);
        state.context.as_mut().unwrap().execution = ExecutionId::new();
    });
    rejects_persisted_change(|state| {
        mark_exited(state);
        state.source_exited = false;
    });
}

#[test]
fn externally_reported_interaction_cannot_become_actionable_by_rewriting_snapshot_storage() {
    rejects_persisted_change(|state| {
        state.last_publication.as_mut().unwrap().state.activity = Activity::WaitingForInput;
        snapshot(state, |fields| {
            fields.activity = Activity::WaitingForInput;
            fields.interaction = Some(
                Interaction::new(
                    InteractionReference {
                        target: fields.cursor.target(),
                        id: uuid::Uuid::new_v4(),
                    },
                    InteractionKind::Text,
                    Vec::new(),
                )
                .unwrap(),
            );
        });
    });
}

#[test]
fn durable_revocation_receipt_cannot_be_relabelled_as_enabled_enrollment() {
    rejects_persisted_change(|state| {
        let expected_revision = state.revision;
        state.revision += super::constants::STEP;
        state.revocation = Some(Revocation {
            request_id: uuid::Uuid::new_v4(),
            expected_revision,
        });
    });
}
