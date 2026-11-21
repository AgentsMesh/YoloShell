use crate::test_isolation_contract::{
    CUSTODY_MARKERS, ISOLATION_CUSTODY, ISOLATION_ROOT, REPEATED_ROOT_DROP, ROOT_MARKERS,
};
use yolo_architecture_test_support::Fixture;

fn accepts(fixture: &Fixture) -> bool {
    crate::inspect(fixture.root()).unwrap().is_empty()
}

#[test]
fn root_owns_one_automatic_cleanup_and_wrapper_delegates_explicit_cleanup() {
    let fixture = Fixture::new();
    fixture.write(ISOLATION_ROOT, &ROOT_MARKERS.join("\n"));
    fixture.write(ISOLATION_CUSTODY, &CUSTODY_MARKERS.join("\n"));
    assert!(accepts(&fixture));
}

#[test]
fn explicit_attempt_and_drop_guard_must_be_executable_code() {
    let original = ROOT_MARKERS.join("\n");
    for marker in ROOT_MARKERS {
        let fixture = Fixture::new();
        fixture.write(
            ISOLATION_ROOT,
            &original.replacen(marker, &format!("/* {marker} */"), 1),
        );
        assert!(!accepts(&fixture), "{marker}");
    }
}

#[test]
fn unguarded_root_drop_and_outer_wrapper_drop_are_rejected() {
    let root = Fixture::new();
    root.write(
        ISOLATION_ROOT,
        &ROOT_MARKERS
            .join("\n")
            .replace("if !self.finalization_attempted", "if true"),
    );
    assert!(!accepts(&root));

    let wrapper = Fixture::new();
    let valid = CUSTODY_MARKERS.join("\n");
    wrapper.write(
        ISOLATION_CUSTODY,
        &format!("{valid}\n{REPEATED_ROOT_DROP} {{}}"),
    );
    assert!(!accepts(&wrapper));
    wrapper.write(
        ISOLATION_CUSTODY,
        &format!("{valid}\n/* {REPEATED_ROOT_DROP} {{}} */"),
    );
    assert!(accepts(&wrapper));
}
