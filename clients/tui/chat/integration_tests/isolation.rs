use super::fixture::*;
use ratatui::{Terminal, backend::TestBackend, layout::Rect};
use yolo_conversation_protocol::{ConversationPart, ConversationRole};
use yolo_tui_chat::{MAX_RENDER_ROWS, Viewport};

const INSET: u16 = 2;
const SENTINEL: &str = "z";
const MANY_ENTRIES: u8 = 32;
const MANY_PARTS: usize = 5;
const BODY_BYTES: usize = 33;
const DRAFT_END: &str = "draft-end";

#[test]
fn pane_rendering_does_not_overwrite_neighbors_and_cached_layouts_follow_width() {
    let (mut chat, _) = ready();
    let transcript = page(&chat, &[1]);
    load(&mut chat, transcript);
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    let area = Rect::new(INSET, INSET, WIDTH - INSET * INSET, HEIGHT - INSET * INSET);
    let mut viewport = Viewport::default();
    terminal
        .draw(|frame| {
            for cell in frame.buffer_mut().content.iter_mut() {
                cell.set_symbol(SENTINEL);
            }
            viewport =
                yolo_tui_chat::render(frame, &chat, area, yolo_tui_chat::InputReadiness::Ready);
        })
        .unwrap();
    assert_eq!(viewport.history.x, area.x);
    for row in 0..HEIGHT {
        for column in 0..WIDTH {
            if !area.contains((column, row).into()) {
                assert_eq!(
                    terminal.backend().buffer()[(column, row)].symbol(),
                    SENTINEL
                );
            }
        }
    }
    assert!(
        super::render_support::render(&chat, WIDTH / INSET, HEIGHT)
            .1
            .total_rows
            > 0
    );
}

#[test]
fn hostile_part_volume_has_a_fixed_render_budget() {
    let (mut chat, _) = ready();
    let ids = (0..MANY_ENTRIES).collect::<Vec<_>>();
    let mut transcript = page(&chat, &ids);
    for entry in &mut transcript.entries {
        entry.role = ConversationRole::Tool;
        entry.parts = vec![
            ConversationPart::Text {
                text: "x".repeat(BODY_BYTES)
            };
            MANY_PARTS
        ];
    }
    load(&mut chat, transcript);
    let (_, viewport) = super::render_support::render(&chat, 1, HEIGHT);
    assert_eq!(viewport.total_rows, MAX_RENDER_ROWS);
    chat.scroll(MAX_RENDER_ROWS, true);
    assert_eq!(
        super::render_support::render(&chat, 1, HEIGHT).1.first_row,
        0
    );
}

#[test]
fn long_multiline_composer_displays_the_caret_end_instead_of_the_history_row_limit() {
    let (mut chat, _) = ready();
    chat.composer
        .insert(&format!("{}{DRAFT_END}", "line\n".repeat(MAX_RENDER_ROWS)))
        .unwrap();
    let (text, _) = super::render_support::render(&chat, WIDTH, HEIGHT);
    assert!(text.contains(DRAFT_END));
}
