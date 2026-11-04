use crate::process_signal_authority_sites::{authority_constructor, qualified, retained_operation};
use crate::process_signal_sites::{named, receiver_site, site};
use yolo_architecture_policy_contracts::process_signal as contract;

#[test]
fn qualified_authority_sites_preserve_every_coordinate_without_substitution() {
    for owner in [None, Some(contract::LIVE_IDENTITY)] {
        let value = qualified(
            contract::PROCESS_SIGNAL_AUTHORITY,
            contract::SIGNAL,
            contract::LIVE_CONTROL_PATH,
            owner,
            contract::VERIFIED,
        );
        assert_eq!(value.qualifier, contract::PROCESS_SIGNAL_AUTHORITY);
        assert_eq!(value.name, contract::SIGNAL);
        assert_eq!(value.site.path, contract::LIVE_CONTROL_PATH);
        assert_eq!(value.site.owner, owner);
        assert_eq!(value.site.function, contract::VERIFIED);
    }
}

#[test]
fn authority_constructors_always_bind_the_factory_to_its_declared_caller() {
    for owner in [None, Some(contract::DESTRUCTIVE_AUTHORITY)] {
        let value = authority_constructor(owner, contract::SIGNAL_EXACT_PROCESS);
        assert_eq!(value.qualifier, contract::PROCESS_SIGNAL_AUTHORITY);
        assert_eq!(value.name, contract::NEW);
        assert_eq!(value.site.path, contract::SIGNAL_AUTHORITY_PATH);
        assert_eq!(value.site.owner, owner);
        assert_eq!(value.site.function, contract::SIGNAL_EXACT_PROCESS);
    }
}

#[test]
fn retained_operations_keep_the_operation_distinct_from_the_authorized_caller() {
    let value = retained_operation(
        contract::SIGNAL,
        contract::LIVE_CONTROL_PATH,
        contract::LIVE_IDENTITY,
        contract::VERIFIED,
    );
    assert_eq!(value.qualifier, contract::RETAINED_EXACT_PROCESS);
    assert_eq!(value.name, contract::SIGNAL);
    assert_eq!(value.site.path, contract::LIVE_CONTROL_PATH);
    assert_eq!(value.site.owner, Some(contract::LIVE_IDENTITY));
    assert_eq!(value.site.function, contract::VERIFIED);
}

#[test]
fn named_and_receiver_sites_do_not_replace_their_nested_site_identity() {
    for owner in [None, Some(contract::PROCESS_SIGNAL_AUTHORITY)] {
        let direct = site(contract::SIGNAL_AUTHORITY_PATH, owner, contract::SIGNAL);
        let named = named(contract::VERIFIED, direct.path, owner, direct.function);
        let receiver = receiver_site(
            contract::CHILD_RECEIVER,
            direct.path,
            owner,
            direct.function,
        );
        assert_eq!(named.name, contract::VERIFIED);
        assert_eq!(receiver.receiver, contract::CHILD_RECEIVER);
        for nested in [named.site, receiver.site] {
            assert_eq!(nested.path, direct.path);
            assert_eq!(nested.owner, owner);
            assert_eq!(nested.function, direct.function);
        }
    }
}
