use crate::support::*;

const TITLE_COMMANDS: [&[u8]; 2] = [b"0", b"2"];
const ENDINGS: [&[u8]; 2] = [BEL, b"\x1b\\"];
const PROGRESS: &[u8] = b"\x1b]9;4;1;75\x07";
const OTHER: &[u8] = b"\x1b]52;c;SGVsbG8=\x07\x1b]7;file://local/tmp\x07\x1b[6n";
const DIRTY_TITLE: &[u8] = b"ok\t\r\n\xe2\x80\xaeTitle";
const CLEAN_TITLE: &str = "okTitle";
const INVALID_UTF8: &[u8] = b"codex\xff";
const REPLACED_UTF8: &str = "codex\u{fffd}";
const COMMAND_TITLE: &[u8] = b"2";
const COMMAND_NOTIFY: &[u8] = b"9";
const COMMAND_URXVT: &[u8] = b"777";
const URXVT_VERB: &[u8] = b"notify;";
const SEMICOLON: &[u8] = b";";

#[test]
fn title_commands_endings_semicolons_and_clear() {
    for command in TITLE_COMMANDS {
        for ending in ENDINGS {
            let result = presentation(&osc(command, TITLE.as_bytes(), ending));
            assert_eq!(result.title.as_deref(), Some(TITLE));
            assert_eq!(result.notification_sequence, u64::default());
        }
    }
    assert_eq!(
        presentation(TITLE_PARTS).title.as_deref(),
        Some(TITLE_PARTS_TEXT)
    );
    assert_eq!(
        presentation(EMPTY_TITLE).title.as_deref(),
        Some(String::new().as_str())
    );
}

#[test]
fn notifications_and_bells_are_distinct_from_osc_terminators() {
    for bytes in [BEL, VISUAL_BELL] {
        let result = presentation(bytes);
        assert_eq!(result.notification_sequence, STEP as u64);
        let notification = result.notification.unwrap();
        assert_eq!(notification.kind, NotificationKind::Bell);
        assert!(notification.title.is_empty() && notification.body.is_empty());
    }
    let result = presentation(NOTIFY);
    assert_eq!(result.notification_sequence, STEP as u64);
    assert_eq!(result.notification.unwrap().body, NOTIFY_BODY);
    let result = presentation(URXVT);
    let notification = result.notification.unwrap();
    assert_eq!(notification.kind, NotificationKind::Desktop);
    assert_eq!(notification.title, URXVT_TITLE);
    assert_eq!(notification.body, URXVT_BODY);
    assert_eq!(
        presentation(&[PROGRESS, OTHER].concat()),
        TerminalPresentation::default()
    );
}

#[test]
fn metadata_is_sanitized_and_bounded_on_utf8_boundaries() {
    assert_eq!(
        presentation(&osc(COMMAND_TITLE, DIRTY_TITLE, BEL))
            .title
            .as_deref(),
        Some(CLEAN_TITLE)
    );
    assert_eq!(
        presentation(&osc(COMMAND_TITLE, INVALID_UTF8, BEL))
            .title
            .as_deref(),
        Some(REPLACED_UTF8)
    );
    let result = presentation(&osc(
        COMMAND_TITLE,
        UNICODE_TEXT.repeat(MAX_TITLE_BYTES).as_bytes(),
        BEL,
    ));
    let title = result.title.unwrap();
    assert!(title.len() <= MAX_TITLE_BYTES);
    assert!(title.len() + UNICODE_TEXT.len() > MAX_TITLE_BYTES);
    let result = presentation(&osc(
        COMMAND_NOTIFY,
        &vec![TEXT_BYTE; MAX_NOTIFICATION_BODY_BYTES + STEP],
        BEL,
    ));
    assert_eq!(
        result.notification.unwrap().body.len(),
        MAX_NOTIFICATION_BODY_BYTES
    );
    let result = presentation(&osc(
        COMMAND_URXVT,
        &[
            URXVT_VERB,
            &vec![TEXT_BYTE; MAX_NOTIFICATION_TITLE_BYTES + STEP],
            SEMICOLON,
            &vec![TEXT_BYTE; MAX_NOTIFICATION_BODY_BYTES + STEP],
        ]
        .concat(),
        BEL,
    ));
    let notification = result.notification.unwrap();
    assert_eq!(notification.title.len(), MAX_NOTIFICATION_TITLE_BYTES);
    assert_eq!(notification.body.len(), MAX_NOTIFICATION_BODY_BYTES);
}

#[test]
fn notification_counter_never_wraps_or_changes_at_exhaustion() {
    let mut value = presentation(NOTIFY);
    value.notification_sequence = u64::MAX;
    let mut parser = vt100::Parser::new_with_callbacks(
        ROWS,
        COLUMNS,
        SCROLLBACK,
        MetadataCallbacks::new(value.clone()),
    );
    parser.process(BEL);
    assert_eq!(parser.callbacks().presentation(), &value);
}
