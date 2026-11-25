use super::test_fixture::Fixture;
use super::*;
use uuid::Uuid;

const SCHEMA_STEP: u16 = 1;

#[test]
fn reboot_intent_header_rejects_each_missing_binding_or_unproven_boot_transition() {
    let fixture = Fixture::new();
    let original = fixture.intent;
    let mutations: &[fn(&mut RebootMigrationIntent)] = &[
        |intent| intent.schema_version += SCHEMA_STEP,
        |intent| intent.transaction_id = Uuid::nil(),
        |intent| intent.terminal_id = Uuid::nil(),
        |intent| intent.runtime_epoch = Uuid::nil(),
        |intent| intent.worker_id = Uuid::nil(),
        |intent| intent.runtime_epoch = intent.worker_id,
        |intent| intent.controller_generation = u64::default(),
        |intent| intent.source_boot_id.clear(),
        |intent| intent.target_boot_id.clear(),
        |intent| intent.target_boot_id.clone_from(&intent.source_boot_id),
        |intent| intent.startup_checkpoint = OptionalParticipant::Absent,
    ];
    assert!(valid_header(&original));
    validate(&original).unwrap();
    for (index, mutate) in mutations.iter().enumerate() {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(!valid_header(&changed), "mutation {index}");
        assert!(validate(&changed).is_err(), "mutation {index}");
    }
}
