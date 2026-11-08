use crate::{constants::INCOMPLETE_COVERAGE, entrypoint_fixture::*};

#[test]
fn partial_profile_inventory_cannot_publish_a_report_or_clear_a_failure_marker() {
    for preexisting_marker in [false, true] {
        let mut fixture = Fixture::new();
        if preexisting_marker {
            std::fs::write(fixture.marker(), FIXTURE_MARKER).unwrap();
        }
        fixture.command.env(FIXTURE_FAILURE_ENV, ENTRY_READ_FAILURE);
        fixture.fails(ENUMERATION_CONTEXT);
        assert!(!fixture.report().exists());
        assert!(!fixture.profile().exists());
        let expected = if preexisting_marker {
            FIXTURE_MARKER
        } else {
            INCOMPLETE_COVERAGE
        };
        assert_eq!(std::fs::read_to_string(fixture.marker()).unwrap(), expected);
        let raw = fixture.root().join(FIXTURE_COVERAGE).join(FIXTURE_RAW);
        assert_eq!(std::fs::read_to_string(raw).unwrap(), FIXTURE_RAW);
    }
}
