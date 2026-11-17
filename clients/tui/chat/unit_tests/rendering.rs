use super::fixture::*;
#[path = "../test_support/render.rs"]
mod support;
use yolo_conversation_protocol::*;
use yolo_tui_chat::*;

#[test]
fn text_tools_reasoning_compaction_and_unsupported_assets_have_explicit_safe_rows() {
    let (mut chat, _) = ready();
    let mut transcript = page(&chat, &[1]);
    transcript.entries[0].parts = vec![
        ConversationPart::Text {
            text: CONTROL_TEXT.into(),
        },
        ConversationPart::Thinking {
            text: "recorded thought".into(),
        },
        ConversationPart::ToolCall {
            call_id: TOOL_CALL.into(),
            name: TOOL_NAME.into(),
            arguments: TOOL_ARGUMENTS.into(),
        },
        ConversationPart::ToolResult {
            call_id: TOOL_CALL.into(),
            output: TOOL_OUTPUT.into(),
            is_error: false,
        },
        ConversationPart::ToolResult {
            call_id: TOOL_CALL.into(),
            output: "permission denied".into(),
            is_error: true,
        },
        ConversationPart::Compaction {
            summary: "earlier context".into(),
        },
        ConversationPart::Notice {
            text: "runtime notice".into(),
        },
        ConversationPart::Unsupported {
            native_kind: UNSUPPORTED_KIND.into(),
        },
        ConversationPart::ImageUnavailable {
            media_type: Some(IMAGE_TYPE.into()),
        },
        ConversationPart::ImageUnavailable { media_type: None },
    ];
    load(&mut chat, transcript);
    let (text, _) = support::render(&chat, WIDTH, HEIGHT);
    for expected in [
        "hello",
        "next    line",
        "Recorded reasoning",
        TOOL_NAME,
        TOOL_CALL,
        TOOL_OUTPUT,
        "Tool failed",
        "Compaction",
        "runtime notice",
        "Unsupported content",
        "Image unavailable",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert!(!text.contains('\u{1b}') && !text.contains('\u{7}'));
    assert_eq!(support::render(&chat, WIDTH, HEIGHT).0, text);
    assert!(!support::render(&chat, 1, HEIGHT).0.contains('\u{1b}'));
}

#[test]
fn empty_partial_loading_and_failed_queries_show_the_actual_content_availability() {
    let snapshot = agent();
    let mut chat = Chat::new(binding(&snapshot)).unwrap();
    assert_eq!(support::render(&chat, WIDTH, HEIGHT).1, Viewport::default());
    chat.set_mode(Mode::Chat);
    let query = chat.resolve().unwrap();
    assert!(
        support::render(&chat, WIDTH, HEIGHT)
            .0
            .contains("Loading conversation")
    );
    chat.resolved(&query, descriptor(binding(&snapshot)));
    let mut empty = page(&chat, &[]);
    empty.next_cursor = None;
    empty.availability = ConversationAvailability::NotYetWritten;
    load(&mut chat, empty.clone());
    assert!(
        support::render(&chat, WIDTH, HEIGHT)
            .0
            .contains("No recorded conversation")
    );
    empty
        .partial
        .push(ConversationPartialReason::IncompleteRecord);
    load(&mut chat, empty);
    assert!(
        support::render(&chat, WIDTH, HEIGHT)
            .0
            .contains("Some conversation content is unavailable")
    );
    for (error, expected) in [
        (QueryError::Unsupported, "conversation reader unavailable"),
        (QueryError::Disconnected, "connection unavailable"),
        (QueryError::InvalidResponse, "did not match"),
        (
            QueryError::Conversation(ConversationError::EvidenceUnavailable),
            "evidence is unavailable",
        ),
    ] {
        let query = chat.read(ReadKind::Latest).unwrap();
        chat.fail(&query, error);
        assert!(support::render(&chat, WIDTH, HEIGHT).0.contains(expected));
    }
    chat.composer.insert(PROMPT).unwrap();
    chat.submit(&snapshot).unwrap();
    assert!(
        support::render(&chat, WIDTH, HEIGHT)
            .0
            .contains("Submitting input")
    );
}

#[test]
fn pending_watch_after_loaded_history_keeps_the_readiness_footer() {
    let (mut chat, _) = ready();
    let transcript = page(&chat, &[1]);
    load(&mut chat, transcript);
    let _watch = chat.read(ReadKind::Latest).unwrap();
    let text = support::render_with_input(&chat, WIDTH, HEIGHT, InputReadiness::Unavailable).0;
    assert!(!text.contains("Loading conversation"), "{text}");
    assert!(text.contains("Agent cannot accept a prompt"), "{text}");
}

#[test]
fn roles_long_parts_unicode_and_narrow_views_stay_bounded() {
    let (mut chat, _) = ready();
    let mut transcript = page(&chat, &[1, 2, 3]);
    transcript.entries[0].role = ConversationRole::User;
    transcript.entries[1].role = ConversationRole::Tool;
    transcript.entries[2].role = ConversationRole::Runtime;
    transcript.entries[2].parts = vec![ConversationPart::Text { text: CJK.into() }];
    load(&mut chat, transcript);
    let (text, view) = support::render(&chat, WIDTH, HEIGHT);
    for role in ["You", "Tool", "Runtime", CJK] {
        assert!(text.contains(role));
    }
    assert!(view.total_rows > 0);
    chat.composer
        .insert(&format!("{CJK}\n{PROMPT}\t{FOLLOW_UP}"))
        .unwrap();
    for (width, height) in [(1, 1), (1, HEIGHT), (WIDTH, 1), (0, 0)] {
        support::render(&chat, width, height);
    }
    let mut big = page(&chat, &[3]);
    big.entries[0].parts = vec![ConversationPart::Text {
        text: "x".repeat(MAX_RENDER_ROWS),
    }];
    load(&mut chat, big);
    let (text, view) = support::render(&chat, WIDTH, HEIGHT);
    assert!(text.contains("more content omitted"));
    assert!(view.total_rows <= MAX_RENDER_ROWS);
    chat.scroll(usize::MAX, true);
    assert_eq!(support::render(&chat, WIDTH, HEIGHT).1.first_row, 0);
}
