use super::fixture::Fixture;
use super::process_signal_test_support::{messages, write_contract};
use yolo_architecture_policy_contracts::process_signal as contract;

const OTHER_PATH: &str = "core/crates/runtime/engine/process/src/unscoped_signal.rs";
const EXACT_SOURCE: &str = r#"
fn signal_process_group_member(group: ProcessGroupIdentity) {
    group.authority.signal(member, signal);
}
"#;
const WRONG_FUNCTION: &str = r#"
fn bypass(group: ProcessGroupIdentity) { group.authority.signal(member, signal); }
"#;
const FORGED_SOURCE: &str = r#"
fn signal_process_group_member() {
    let group = ProcessGroupIdentity::verified(group_id, arbitrary_members);
    group.authority.signal(member, signal);
}
"#;

#[test]
fn exact_member_signal_site_does_not_authorize_other_functions_or_paths() {
    let valid = Fixture::new();
    write_contract(&valid);
    valid.write(contract::INVENTORY_MEMBER_SIGNAL_PATH, EXACT_SOURCE);
    let result = messages(&valid);
    assert!(!result.iter().any(|message| {
        message.contains(contract::AUTHORIZED_ENTRY_MESSAGE)
            || message.contains(contract::AUTHORITY_TYPE_MESSAGE)
    }));
    for (path, source) in [
        (contract::INVENTORY_MEMBER_SIGNAL_PATH, WRONG_FUNCTION),
        (OTHER_PATH, EXACT_SOURCE),
    ] {
        let fixture = Fixture::new();
        write_contract(&fixture);
        fixture.write(path, source);
        assert!(
            messages(&fixture)
                .iter()
                .any(|message| message.contains(contract::AUTHORIZED_ENTRY_MESSAGE))
        );
    }
}

#[test]
fn exact_member_sender_cannot_construct_its_own_group_authority() {
    let fixture = Fixture::new();
    write_contract(&fixture);
    fixture.write(contract::INVENTORY_MEMBER_SIGNAL_PATH, FORGED_SOURCE);
    assert!(
        messages(&fixture)
            .iter()
            .any(|message| message.contains(contract::AUTHORITY_CONSTRUCTOR_MESSAGE))
    );
}
