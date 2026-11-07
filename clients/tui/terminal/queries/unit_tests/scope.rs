use crate::fixtures::*;
use yolo_tui_terminal_queries::QueryScope;

#[test]
fn nil_epoch_cannot_construct_a_query_scope() {
    assert_eq!(QueryScope::new(uuid::Uuid::nil()), None);
}

#[test]
fn partial_query_cannot_cross_writer_epoch_or_quiet_replay() {
    for quiet in [false, true] {
        let mut fixture = Fixture::new();
        assert!(fixture.live(PARTIAL_CPR).is_empty());
        if quiet {
            fixture.quiet(&[]);
        } else {
            fixture.scope = scope();
        }
        assert!(fixture.live(CPR_FINAL).is_empty());
        assert_eq!(fixture.live(CPR), [HOME_REPORT.to_vec()]);
    }
}

#[test]
fn observer_partial_query_and_explicit_reset_cannot_produce_a_new_writer_reply() {
    let mut fixture = Fixture::new();
    fixture.quiet(PARTIAL_CPR);
    assert!(fixture.live(CPR_FINAL).is_empty());
    assert!(fixture.live(PARTIAL_CPR).is_empty());
    fixture.queries.reset();
    assert!(fixture.live(CPR_FINAL).is_empty());
    assert_eq!(fixture.live(CPR), [HOME_REPORT.to_vec()]);
}

#[test]
fn absent_authority_and_scope_changes_preserve_partial_utf8_screen_state() {
    let mut fixture = Fixture::new();
    let bytes = WIDE_TEXT.as_bytes();
    assert!(fixture.live(&bytes[..SPLIT_STEP]).is_empty());
    assert!(
        fixture
            .queries
            .process(&mut fixture.parser, &bytes[SPLIT_STEP..], None)
            .is_empty()
    );
    assert_eq!(fixture.parser.screen().contents(), WIDE_TEXT);
    assert!(
        fixture
            .queries
            .process(&mut fixture.parser, DSR, None)
            .is_empty()
    );
    assert_eq!(fixture.live(DSR), [STATUS.to_vec()]);
}
