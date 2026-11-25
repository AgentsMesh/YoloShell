use super::{
    fixtures::{COLUMNS, ROWS, SCROLLBACK},
    osc_support::*,
};

#[test]
fn public_vte_dispatches_below_and_exact_raw_limit_but_never_an_oversize_prefix() {
    for raw_bytes in [
        vte::MAX_OSC_RAW_BYTES - STEP,
        vte::MAX_OSC_RAW_BYTES,
        vte::MAX_OSC_RAW_BYTES + STEP,
    ] {
        for termination in [BEL, CAN, SUB, ST] {
            let mut parser = vte::Parser::new();
            let mut collector = OscCollector::default();
            parser.advance(&mut collector, &title(raw_bytes, termination));
            if raw_bytes <= vte::MAX_OSC_RAW_BYTES && !matches!(termination, CAN | SUB) {
                assert_eq!(collector.records.len(), STEP);
                let record = &collector.records[usize::default()];
                assert_eq!(record[usize::default()], TITLE_PARAMETER);
                assert_eq!(record.iter().map(Vec::len).sum::<usize>(), raw_bytes);
            } else {
                assert!(collector.records.is_empty());
            }
            collector.records.clear();
            parser.advance(&mut collector, NEXT_TITLE);
            assert_eq!(
                collector.records,
                vec![vec![TITLE_PARAMETER.to_vec(), NEXT_TITLE_TEXT.to_vec()]]
            );
        }
    }
}

#[test]
fn separators_do_not_consume_raw_payload_budget_and_clipboard_is_not_partially_dispatched() {
    for raw_bytes in [vte::MAX_OSC_RAW_BYTES, vte::MAX_OSC_RAW_BYTES + STEP] {
        let mut parser = vte::Parser::new();
        let mut collector = OscCollector::default();
        let bytes = clipboard(raw_bytes, BEL);
        for chunk in bytes.chunks(CHUNK_BYTES) {
            parser.advance(&mut collector, chunk);
        }
        if raw_bytes == vte::MAX_OSC_RAW_BYTES {
            assert_eq!(collector.records.len(), STEP);
            let record = &collector.records[usize::default()];
            assert_eq!(record.iter().map(Vec::len).sum::<usize>(), raw_bytes);
            assert_eq!(record[usize::default()], CLIPBOARD_PARAMETER);
            assert_eq!(record[STEP], CLIPBOARD_SELECTOR);
        } else {
            assert!(collector.records.is_empty());
        }
    }
}

#[test]
fn vt100_never_receives_truncated_oversize_title_or_clipboard_callbacks() {
    let mut parser =
        vt100::Parser::new_with_callbacks(ROWS, COLUMNS, SCROLLBACK, ScreenCallbacks::default());
    for bytes in [
        title(vte::MAX_OSC_RAW_BYTES + STEP, BEL),
        clipboard(vte::MAX_OSC_RAW_BYTES + STEP, ST),
    ] {
        for chunk in bytes.chunks(CHUNK_BYTES) {
            parser.process(chunk);
        }
    }
    assert!(parser.callbacks().titles.is_empty());
    assert!(parser.callbacks().clipboard.is_empty());
    parser.process(NEXT_TITLE);
    parser.process(NEXT_CLIPBOARD);
    assert_eq!(parser.callbacks().titles, [NEXT_TITLE_TEXT.to_vec()]);
    assert_eq!(parser.callbacks().clipboard, [NEXT_CLIPBOARD_TEXT.to_vec()]);
}
