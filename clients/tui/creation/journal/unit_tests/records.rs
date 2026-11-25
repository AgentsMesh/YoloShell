use super::support::*;
use crate::{PendingFinalization, constants::*, record::Document};
use yolo_hub_identity::HubId;

pub(super) fn document() -> Document {
    Document {
        schema_version: JOURNAL_SCHEMA_VERSION,
        entry_hub: HubId::new(),
        directory: directory_identity(),
        revision: FIRST_REVISION,
        pending: vec![pending(HubId::new())],
        intents: Vec::new(),
    }
}

#[test]
fn confirmed_receipt_keeps_remote_origin_and_rejects_substituted_result_origin() {
    let record = pending(HubId::new());
    record.validate().unwrap();
    assert!(PendingFinalization::new(HubId::new(), record.request, record.creation).is_err());
}

#[test]
fn document_binds_exact_entry_namespace_version_and_nonzero_revision() {
    let original = document();
    original
        .validate(original.entry_hub, original.directory)
        .unwrap();
    assert!(original.validate(HubId::new(), original.directory).is_err());
    assert!(
        original
            .validate(original.entry_hub, directory_identity())
            .is_err()
    );
    let mut changed = original.clone();
    changed.schema_version += SCHEMA_STEP;
    assert!(
        changed
            .validate(original.entry_hub, original.directory)
            .is_err()
    );
    changed = original.clone();
    changed.revision = u64::default();
    assert!(
        changed
            .validate(original.entry_hub, original.directory)
            .is_err()
    );
    changed = original.clone();
    changed.pending[FIRST_INDEX].origin = HubId::new();
    assert!(
        changed
            .validate(original.entry_hub, original.directory)
            .is_err()
    );
}

#[test]
fn request_token_and_session_aliases_are_rejected_even_when_fields_differ() {
    let original = document();
    let retained = &original.pending[FIRST_INDEX];
    let variants = [
        retained.clone(),
        {
            let mut value = pending(retained.origin);
            change_request(&mut value, |fields| {
                fields.request_id = retained.request.fields().request_id
            });
            value
        },
        {
            let mut value = pending(retained.origin);
            change_request(&mut value, |fields| {
                fields.creation_token = retained.request.fields().creation_token
            });
            value
        },
        {
            let mut value = pending(retained.origin);
            value.creation.address = retained.creation.address;
            value
        },
    ];
    for value in variants {
        let mut changed = original.clone();
        changed.pending.push(value);
        assert!(
            changed
                .validate(original.entry_hub, original.directory)
                .is_err()
        );
    }
}

#[test]
fn request_and_token_collisions_are_rejected_across_origins_before_restore() {
    for same_token in [false, true] {
        let mut value = document();
        let mut remote = pending(HubId::new());
        let original = value.pending[FIRST_INDEX].request.fields();
        change_request(&mut remote, |fields| {
            if same_token {
                fields.creation_token = original.creation_token;
            } else {
                fields.request_id = original.request_id;
            }
        });
        value.pending.push(remote);
        assert!(value.validate(value.entry_hub, value.directory).is_err());
    }
}

#[test]
fn record_count_is_bounded_at_the_declared_creation_limit() {
    let mut value = document();
    value.pending = (0..MAX_PENDING_CREATIONS)
        .map(|_| pending(HubId::new()))
        .collect();
    value.validate(value.entry_hub, value.directory).unwrap();
    value.pending.push(pending(HubId::new()));
    assert!(value.validate(value.entry_hub, value.directory).is_err());
}
