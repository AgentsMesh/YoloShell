use super::{constants::*, fixture::*};

#[test]
fn two_slots_recover_either_side_of_configuration_commit() {
    let mut fixture = Fixture::new(true);
    let before = fixture.configuration.current().unwrap();
    let mut bindings = fixture.open(true).unwrap();
    bindings
        .establish(&before, ALLOCATED.parse().unwrap())
        .unwrap();
    let change = fixture.change();
    let planned = fixture.configuration.planned(&change).unwrap();
    bindings
        .prepare(&before, &planned, Some(ALLOCATED.parse().unwrap()))
        .unwrap();
    let prepared = std::fs::read(fixture.path()).unwrap();
    drop(bindings);
    let bindings = fixture.open(false).unwrap();
    assert_eq!(
        bindings.address(&before).unwrap(),
        Some(ALLOCATED.parse().unwrap())
    );
    drop(bindings);
    assert_eq!(
        fixture.configuration.apply(change.clone()).unwrap(),
        planned
    );
    let bindings = fixture.open(false).unwrap();
    assert_eq!(
        bindings.address(&planned).unwrap(),
        Some(ALLOCATED.parse().unwrap())
    );
    assert_eq!(std::fs::read(fixture.path()).unwrap(), prepared);
    assert_eq!(fixture.configuration.apply(change).unwrap(), planned);
}

#[test]
fn explicit_disabled_and_nonzero_configurations_cannot_reactivate_stale_allocations() {
    for requested in [None, Some(NEXT_ALLOCATED.parse().unwrap())] {
        let mut fixture = Fixture::new(true);
        let before = fixture.configuration.current().unwrap();
        let mut bindings = fixture.open(true).unwrap();
        bindings
            .establish(&before, ALLOCATED.parse().unwrap())
            .unwrap();
        let mut change = fixture.change();
        change.configuration.listen = requested;
        let explicit = fixture.configuration.planned(&change).unwrap();
        bindings.prepare(&before, &explicit, requested).unwrap();
        fixture.configuration.apply(change).unwrap();
        drop(bindings);
        let mut bindings = fixture.open(false).unwrap();
        assert_eq!(bindings.address(&explicit).unwrap(), None);
        let mut change = fixture.change();
        change.configuration.listen = Some(LOOPBACK.parse().unwrap());
        let next = fixture.configuration.planned(&change).unwrap();
        assert!(bindings.prepare(&explicit, &next, None).is_err());
        bindings
            .prepare(&explicit, &next, Some(NEXT_ALLOCATED.parse().unwrap()))
            .unwrap();
        fixture.configuration.apply(change).unwrap();
        drop(bindings);
        let bindings = fixture.open(false).unwrap();
        assert_eq!(
            bindings.address(&next).unwrap(),
            Some(NEXT_ALLOCATED.parse().unwrap())
        );
        assert!(bindings.address(&before).is_err());
    }
}
