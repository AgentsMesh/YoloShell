use super::{Key, submission, toggle};
use crate::{Mode, State, network::Job};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use tokio::sync::mpsc;

pub(crate) fn handle(state: &mut State, jobs: &mpsc::Sender<Job>, event: &Event) -> bool {
    if state.mode != Mode::Chat {
        return false;
    }
    if let Event::Key(key) = event {
        if key.kind == KeyEventKind::Release {
            return true;
        }
        if key.code == KeyCode::Enter && key.modifiers.is_empty() {
            if key.kind == KeyEventKind::Press {
                submission::send(state, jobs);
            }
            return true;
        }
        if key.code == KeyCode::Esc {
            toggle(state);
            return true;
        }
    }
    let key = Key::current(state);
    let Some(entry) = state.chats.entries.get_mut(&key) else {
        return true;
    };
    let discard = matches!(event, Event::Key(key) if key.code == KeyCode::Char('x') && key.modifiers == KeyModifiers::CONTROL);
    let navigation = matches!(event, Event::Key(key) if matches!(key.code,
        KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown));
    if entry.view.retired()
        && !discard
        && !navigation
        && matches!(event, Event::Key(_) | Event::Paste(_))
    {
        state.status = super::constants::RETIRED.into();
        return true;
    }
    let result = match event {
        Event::Paste(text) => entry.view.composer.insert(text),
        Event::Key(key) => match key.code {
            KeyCode::Char('x') if key.modifiers == KeyModifiers::CONTROL => {
                if entry.view.composer.discard_uncertain() {
                    Ok(true)
                } else {
                    entry.view.composer.clear().map(|_| true)
                }
            }
            KeyCode::Char(character)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                entry.view.composer.insert(&character.to_string())
            }
            KeyCode::Enter => entry.view.composer.insert("\n"),
            KeyCode::Tab => entry.view.composer.insert("\t"),
            KeyCode::Backspace => entry.view.composer.backspace(),
            KeyCode::Delete => entry.view.composer.delete(),
            KeyCode::Left => {
                entry.view.composer.left();
                Ok(false)
            }
            KeyCode::Right => {
                entry.view.composer.right();
                Ok(false)
            }
            KeyCode::Home => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    entry.view.composer.document_start();
                } else {
                    entry.view.composer.home();
                }
                Ok(false)
            }
            KeyCode::End => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    entry.view.composer.document_end();
                } else {
                    entry.view.composer.end();
                }
                Ok(false)
            }
            KeyCode::PageUp => {
                entry
                    .view
                    .scroll(crate::constants::SCROLL_ROWS_PER_TICK, true);
                if !entry.view.retired() {
                    entry.older = true;
                }
                Ok(false)
            }
            KeyCode::PageDown => {
                entry
                    .view
                    .scroll(crate::constants::SCROLL_ROWS_PER_TICK, false);
                Ok(false)
            }
            _ => Ok(false),
        },
        Event::Mouse(mouse)
            if matches!(
                mouse.kind,
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
            ) =>
        {
            entry.view.scroll(
                crate::constants::SCROLL_ROWS_PER_TICK,
                mouse.kind == MouseEventKind::ScrollUp,
            );
            Ok(false)
        }
        _ => return false,
    };
    if let Err(error) = result {
        state.status = error.to_string();
    }
    let discarded = discard
        && entry.view.retired()
        && entry.view.composer.text().is_empty()
        && !entry.view.composer.locked();
    if discarded {
        state.chats.entries.remove(&key);
        state.mode = Mode::Terminal;
        toggle(state);
    }
    true
}
