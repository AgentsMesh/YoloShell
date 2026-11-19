use crate::{InputError, constants::*};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

pub fn encode_key(key: KeyEvent, screen: &vt100::Screen) -> Result<Vec<u8>, InputError> {
    encode_key_modes(key, screen.application_keypad(), screen.application_cursor())
}

/// Encode a key before the first PTY snapshot is available. New shells start
/// with the normal terminal modes; later snapshots supersede this fallback for
/// application cursor and keypad modes.
pub fn encode_key_default(key: KeyEvent) -> Result<Vec<u8>, InputError> {
    encode_key_modes(key, false, false)
}

fn encode_key_modes(
    key: KeyEvent,
    application_keypad: bool,
    application_cursor: bool,
) -> Result<Vec<u8>, InputError> {
    if key.kind == KeyEventKind::Release {
        return Ok(Vec::new());
    }
    let unsupported = KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META;
    if key.modifiers.intersects(unsupported) {
        return Err(InputError::UnsupportedKey);
    }
    if let Some(keypad) = crate::keypad::encode(key, application_keypad)? {
        return Ok(keypad);
    }
    if let Some(sequence) = special(key, application_cursor)? {
        return Ok(sequence);
    }
    let mut output = Vec::new();
    if key.modifiers.contains(KeyModifiers::ALT) {
        output.push(ESCAPE);
    }
    match key.code {
        KeyCode::Char(character) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let normalized = character.to_ascii_uppercase();
            if normalized == '?' {
                output.push(DELETE);
            } else if normalized == ' ' || ('@'..='_').contains(&normalized) {
                output.push(normalized as u8 & CONTROL_MASK);
            } else {
                return Err(InputError::UnsupportedKey);
            }
        }
        KeyCode::Char(character) => {
            let mut buffer = [0; std::mem::size_of::<char>()];
            output.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
        }
        KeyCode::Enter => output.push(CARRIAGE_RETURN),
        KeyCode::Backspace => output.push(DELETE),
        KeyCode::Tab => output.push(TAB),
        KeyCode::Esc => output.push(ESCAPE),
        _ => return Err(InputError::UnsupportedKey),
    }
    Ok(output)
}

fn modifier(modifiers: KeyModifiers) -> u8 {
    MODIFIER_BASE
        + u8::from(modifiers.contains(KeyModifiers::SHIFT)) * SHIFT_MODIFIER
        + u8::from(modifiers.contains(KeyModifiers::ALT)) * ALT_MODIFIER
        + u8::from(modifiers.contains(KeyModifiers::CONTROL)) * CONTROL_MODIFIER
}

fn special(key: KeyEvent, application_cursor: bool) -> Result<Option<Vec<u8>>, InputError> {
    let suffix = match key.code {
        KeyCode::Up => Some('A'),
        KeyCode::Down => Some('B'),
        KeyCode::Right => Some('C'),
        KeyCode::Left => Some('D'),
        KeyCode::Home => Some('H'),
        KeyCode::End => Some('F'),
        _ => None,
    };
    let mode = modifier(key.modifiers);
    if let Some(suffix) = suffix {
        let value = if mode != MODIFIER_BASE {
            format!("{MODIFIED_CURSOR}{mode}{suffix}")
        } else {
            format!(
                "{}{suffix}",
                if application_cursor {
                    APPLICATION_CURSOR
                } else {
                    NORMAL_CURSOR
                }
            )
        };
        return Ok(Some(value.into_bytes()));
    }
    let code = match key.code {
        KeyCode::Insert => Some(INSERT_CODE),
        KeyCode::Delete => Some(DELETE_CODE),
        KeyCode::PageUp => Some(PAGE_UP_CODE),
        KeyCode::PageDown => Some(PAGE_DOWN_CODE),
        KeyCode::F(number) if (FIRST_FUNCTION_KEY..=SS3_FUNCTION_COUNT).contains(&number) => {
            let suffix = char::from(FIRST_SS3_FUNCTION + number - FIRST_FUNCTION_KEY);
            let value = if mode == MODIFIER_BASE {
                format!("{APPLICATION_CURSOR}{suffix}")
            } else {
                format!("{MODIFIED_CURSOR}{mode}{suffix}")
            };
            return Ok(Some(value.into_bytes()));
        }
        KeyCode::F(number) if number > SS3_FUNCTION_COUNT => Some(
            *FUNCTION_CODES
                .get(usize::from(
                    number - SS3_FUNCTION_COUNT - FIRST_FUNCTION_KEY,
                ))
                .ok_or(InputError::UnsupportedKey)?,
        ),
        KeyCode::F(_) => return Err(InputError::UnsupportedKey),
        KeyCode::BackTab => return Ok(Some(BACK_TAB.as_bytes().to_vec())),
        _ => None,
    };
    Ok(code.map(|code| {
        if mode == MODIFIER_BASE {
            format!("{NORMAL_CURSOR}{code}~").into_bytes()
        } else {
            format!("{NORMAL_CURSOR}{code};{mode}~").into_bytes()
        }
    }))
}
