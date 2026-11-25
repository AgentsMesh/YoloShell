use crate::support::*;

const COMMAND_TITLE: &[u8] = b"2";
const ST: &[u8] = b"\x1b\\";
const CANCEL: &[u8] = b"\x18";
const START_TITLE: &[u8] = b"\x1b]2;";
const SCREEN_TEXT: &[u8] = b"screen";
const EXPECTED_SCREEN: &str = "screen";
const INTERRUPTING_CSI: &[u8] = b"\x1b[6n";

#[test]
fn fragmented_utf8_and_osc_delimiters_produce_exactly_one_effect() {
    for bytes in [
        osc(COMMAND_TITLE, TITLE.as_bytes(), ST),
        TITLE_PARTS.to_vec(),
        NOTIFY.to_vec(),
        URXVT.to_vec(),
    ] {
        let expected = presentation(&bytes);
        for split in usize::default()..=bytes.len() {
            let mut parser = parser();
            parser.process(&bytes[..split]);
            if split < bytes.len() {
                assert_eq!(
                    parser.callbacks().presentation(),
                    &TerminalPresentation::default()
                );
            }
            parser.process(&bytes[split..]);
            assert_eq!(parser.callbacks().presentation(), &expected);
        }
        let mut parser = parser();
        for byte in &bytes {
            parser.process(std::slice::from_ref(byte));
        }
        assert_eq!(parser.callbacks().presentation(), &expected);
    }
}

#[test]
fn snapshot_restores_title_without_replaying_any_notification() {
    let mut original = parser();
    original.process(&[TITLE_PARTS, NOTIFY, BEL, SCREEN_TEXT].concat());
    let mut restored = parser();
    let mut snapshot = original.callbacks().snapshot_prefix(false);
    snapshot.extend(original.screen().state_formatted());
    restored.process(&snapshot);
    assert_eq!(
        restored.callbacks().presentation().title,
        original.callbacks().presentation().title
    );
    assert_eq!(
        restored.callbacks().presentation().notification_sequence,
        u64::default()
    );
    assert!(restored.callbacks().presentation().notification.is_none());
    assert_eq!(restored.screen().contents(), EXPECTED_SCREEN);
    assert_eq!(
        parser().callbacks().presentation(),
        &TerminalPresentation::default()
    );
}

#[test]
fn snapshot_of_the_alternate_screen_keeps_its_mode_and_leaves_cleanly() {
    let mut original = parser();
    original.process(b"\x1b[?1049hALT");
    assert!(original.screen().alternate_screen());
    let mut restored = parser();
    let mut snapshot = original
        .callbacks()
        .snapshot_prefix(original.screen().alternate_screen());
    snapshot.extend(original.screen().state_formatted());
    restored.process(&snapshot);
    assert!(restored.screen().alternate_screen());
    assert_eq!(restored.screen().contents().trim(), "ALT");
    restored.process(b"\x1b[?1049l");
    assert!(!restored.screen().alternate_screen());
    assert!(!restored.screen().contents().contains("ALT"));
}

#[test]
fn oversized_unterminated_cancelled_and_nested_controls_never_publish_partial_metadata() {
    for ending in [BEL, ST, CANCEL] {
        let mut parser = parser();
        parser.process(&osc(
            COMMAND_TITLE,
            &vec![TEXT_BYTE; vte::MAX_OSC_RAW_BYTES + STEP],
            ending,
        ));
        assert_eq!(
            parser.callbacks().presentation(),
            &TerminalPresentation::default()
        );
        parser.process(NOTIFY);
        assert_eq!(
            parser.callbacks().presentation().notification_sequence,
            STEP as u64
        );
    }
    let mut parser = parser();
    parser.process(&[START_TITLE, TITLE.as_bytes()].concat());
    assert_eq!(
        parser.callbacks().presentation(),
        &TerminalPresentation::default()
    );
    parser.process(CANCEL);
    assert_eq!(
        parser.callbacks().presentation(),
        &TerminalPresentation::default()
    );
    parser.process(&[START_TITLE, TITLE.as_bytes(), INTERRUPTING_CSI].concat());
    assert_eq!(
        parser.callbacks().presentation(),
        &TerminalPresentation::default()
    );
}
