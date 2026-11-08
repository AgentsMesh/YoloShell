use crate::{InputError, constants::APPLICATION_CURSOR};
use crossterm::event::{KeyCode, KeyEvent, KeyEventState};

pub(crate) fn encode(key: KeyEvent, application: bool) -> Result<Option<Vec<u8>>, InputError> {
    if !application || !key.state.contains(KeyEventState::KEYPAD) {
        return Ok(None);
    }
    if !key.modifiers.is_empty() {
        return Err(InputError::UnsupportedKey);
    }
    let suffix = match key.code {
        KeyCode::Char('0') => 'p',
        KeyCode::Char('1') => 'q',
        KeyCode::Char('2') => 'r',
        KeyCode::Char('3') => 's',
        KeyCode::Char('4') => 't',
        KeyCode::Char('5') => 'u',
        KeyCode::Char('6') => 'v',
        KeyCode::Char('7') => 'w',
        KeyCode::Char('8') => 'x',
        KeyCode::Char('9') => 'y',
        KeyCode::Char('.') => 'n',
        KeyCode::Char(',') => 'l',
        KeyCode::Char('+') => 'k',
        KeyCode::Char('-') => 'm',
        KeyCode::Char('*') => 'j',
        KeyCode::Char('/') => 'o',
        KeyCode::Char('=') => 'X',
        KeyCode::Enter => 'M',
        _ => return Err(InputError::UnsupportedKey),
    };
    Ok(Some(format!("{APPLICATION_CURSOR}{suffix}").into_bytes()))
}
