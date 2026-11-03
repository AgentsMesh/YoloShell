use crate::fixtures::*;

#[test]
fn every_query_split_across_a_new_authority_epoch_is_silent() {
    for query in [DSR, CPR, DA_ZERO] {
        for split in SPLIT_STEP..query.len() {
            let mut fixture = Fixture::new();
            assert!(fixture.live(&query[..split]).is_empty());
            fixture.scope = scope();
            assert!(fixture.live(&query[split..]).is_empty());
            assert_eq!(fixture.live(DSR), [STATUS.to_vec()]);
        }
    }
}

#[test]
fn cancellation_and_osc_or_dcs_content_do_not_manufacture_eligible_queries() {
    for query in IGNORE_QUERIES {
        for split in SPLIT_STEP..query.len() {
            let mut fixture = Fixture::new();
            fixture.quiet(&query[..split]);
            assert!(fixture.live(&query[split..]).is_empty());
        }
    }
}

#[test]
fn query_candidate_cannot_override_an_independent_screen_parser_cancellation() {
    for cancel in [CANCEL_SEQUENCE, SUBSTITUTE_SEQUENCE, RESET_SCREEN] {
        let mut fixture = Fixture::new();
        assert!(fixture.live(PARTIAL_CPR).is_empty());
        // The public processor accepts an externally owned vt100 parser. If its
        // owner feeds a cancellation directly, eligibility alone must not turn
        // the subsequent ordinary text into a terminal query reply.
        fixture.parser.process(cancel);
        assert!(fixture.live(CPR_FINAL).is_empty());
        assert_eq!(fixture.parser.screen().contents(), UNMATCHED_FINAL_TEXT);
        // Both parsers recognize a later complete query, without replaying the
        // canceled candidate or resetting the continuously rendered screen.
        assert_eq!(fixture.live(DSR), [STATUS.to_vec()]);
        assert_eq!(fixture.parser.screen().contents(), UNMATCHED_FINAL_TEXT);
    }
}
