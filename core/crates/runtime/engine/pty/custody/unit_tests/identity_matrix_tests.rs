use super::super::tests::{descriptor_set, digest, request};
use super::*;

const INVALID_PROTOCOL_VERSION: u16 = u16::MAX;

#[test]
fn every_runtime_identity_field_is_required_for_custody_admission() {
    let original = request();
    let mutations: &[fn(&mut CustodyRuntimeIdentity)] = &[
        |value| value.terminal_id = Uuid::nil(),
        |value| value.runtime_epoch = Uuid::nil(),
        |value| value.worker_id = Uuid::nil(),
        |value| value.worker_pid = u32::default(),
        |value| value.worker_start_time = u64::default(),
        |value| value.child_pid = u32::default(),
        |value| value.child_start_time = u64::default(),
        |value| value.boot_id.clear(),
    ];
    assert!(original.runtime.is_complete());
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original.clone();
        mutate(&mut changed.runtime);
        assert!(!changed.runtime.is_complete(), "mutation {index}");
        assert!(!changed.is_well_formed(), "mutation {index}");
    }
}

#[test]
fn custody_request_rejects_missing_holder_token_generation_and_unknown_protocol() {
    let original = request();
    let offer = super::super::CustodyOffer::new(&original, descriptor_set(), digest());
    let pending = super::super::CustodyDurableRecord::pending(original.runtime.clone());
    let mutations: &[fn(&mut super::super::CustodyRequest)] = &[
        |value| value.protocol_version = INVALID_PROTOCOL_VERSION,
        |value| value.holder.pid = u32::default(),
        |value| value.holder.start_time = u64::default(),
        |value| value.auth_token.clear(),
        |value| value.controller_generation = u64::default(),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(!changed.is_well_formed(), "mutation {index}");
        assert!(
            super::super::arm_fixture(&pending, &changed, &offer).is_none(),
            "mutation {index}"
        );
    }
    assert!(original.is_well_formed());
    assert!(offer.matches_request(&original));
}
