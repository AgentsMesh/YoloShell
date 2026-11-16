use crate::fixtures::*;
use yolo_tui_terminal_queries::QueryCallbacks;

#[test]
fn synchronized_output_mode_tracks_set_and_reset_across_fragmented_chunks() {
    let mut parser =
        vt100::Parser::new_with_callbacks(ROWS, COLUMNS, SCROLLBACK, QueryCallbacks::default());
    assert!(!parser.callbacks().synchronized_output());
    for (bytes, expected) in [
        (b"\x1b[?2026".as_slice(), false),
        (b"h", true),
        (b"intermediate output", true),
        (b"\x1b[?2026l", false),
    ] {
        parser.process(bytes);
        assert_eq!(parser.callbacks().synchronized_output(), expected);
    }
}

#[test]
fn standard_queries_use_exact_screen_state_at_each_dispatch() {
    let mut fixture = Fixture::new();
    assert_eq!(
        fixture.live(ORDERED_QUERIES),
        [THIRD_REPORT, FOURTH_REPORT, STATUS, ATTRIBUTES].map(<[u8]>::to_vec)
    );
    assert_eq!(fixture.live(DA_ZERO), [ATTRIBUTES.to_vec()]);
}

#[test]
fn every_fragmentation_boundary_preserves_one_reply_in_the_same_scope() {
    for bytes in [DSR, CPR, DA, DA_ZERO] {
        for split in SPLIT_STEP..bytes.len() {
            let mut fixture = Fixture::new();
            let mut replies = fixture.live(&bytes[..split]);
            replies.extend(fixture.live(&bytes[split..]));
            let expected = if bytes == DSR {
                STATUS
            } else if bytes == CPR {
                HOME_REPORT
            } else {
                ATTRIBUTES
            };
            assert_eq!(replies, [expected.to_vec()]);
        }
    }
}

#[test]
fn private_extended_malformed_and_ignored_sequences_do_not_reply() {
    for bytes in IGNORE_QUERIES
        .iter()
        .copied()
        .chain([PARAMETER_OVERFLOW, INTERMEDIATE_OVERFLOW])
    {
        let mut fixture = Fixture::new();
        assert!(fixture.live(bytes).is_empty());
        assert_eq!(fixture.live(DSR), [STATUS.to_vec()]);
    }
}

#[test]
fn bounded_candidates_cannot_truncate_screen_processing() {
    let mut fixture = Fixture::new();
    let mut bytes = DSR.to_vec();
    bytes.extend_from_slice(OUTPUT_SUFFIX);
    assert_eq!(fixture.live(&bytes), [STATUS.to_vec()]);
    assert_eq!(fixture.parser.screen().contents(), OUTPUT_SUFFIX_TEXT);
}
