use crate::support::*;
use yolo_terminal_metadata::title_from_snapshot;

const TITLE_COMMAND: &[u8] = b"2";
const OTHER_COMMAND: &[u8] = b"0";
const ST: &[u8] = b"\x1b\\";
const BOGUS_UTF8: &[u8] = b"title\xff";
const SCREEN_PREFIX: &[u8] = b"\x1b[m";
const LARGE_SCREEN_BYTES: usize = 1024 * 1024;

#[test]
fn snapshot_title_reads_only_current_prefix_and_ignores_screen_notifications() {
    let mut snapshot = osc(TITLE_COMMAND, TITLE.as_bytes(), ST);
    snapshot.extend(vec![TEXT_BYTE; LARGE_SCREEN_BYTES]);
    snapshot.extend(NOTIFY);
    snapshot.extend(TITLE_PARTS);
    assert_eq!(title_from_snapshot(&snapshot).as_deref(), Some(TITLE));
    assert_eq!(
        title_from_snapshot(&osc(TITLE_COMMAND, &[], ST)),
        Some(String::new())
    );
    let title = vec![TEXT_BYTE; MAX_TITLE_BYTES];
    assert_eq!(
        title_from_snapshot(&osc(TITLE_COMMAND, &title, ST))
            .unwrap()
            .len(),
        MAX_TITLE_BYTES
    );
}

#[test]
fn snapshot_title_rejects_missing_malformed_oversize_and_noncurrent_prefixes() {
    for bytes in [
        osc(OTHER_COMMAND, TITLE.as_bytes(), ST),
        osc(TITLE_COMMAND, TITLE.as_bytes(), BEL),
        osc(TITLE_COMMAND, TITLE.as_bytes(), &[]),
        osc(TITLE_COMMAND, BOGUS_UTF8, ST),
        osc(TITLE_COMMAND, ESCAPE_INJECTION.as_bytes(), ST),
        osc(TITLE_COMMAND, BIDI_INJECTION.as_bytes(), ST),
        osc(TITLE_COMMAND, &vec![TEXT_BYTE; MAX_TITLE_BYTES + STEP], ST),
        [SCREEN_PREFIX, TITLE_PARTS].concat(),
        Vec::new(),
    ] {
        assert_eq!(title_from_snapshot(&bytes), None);
    }
}
