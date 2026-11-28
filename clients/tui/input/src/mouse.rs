use crate::{InputError, mouse_constants::*};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use vt100::{MouseProtocolEncoding, MouseProtocolMode};

/// Coordinates are zero-based cells in the source terminal, not the outer TUI.
pub fn encode_mouse(event: MouseEvent, screen: &vt100::Screen) -> Result<Vec<u8>, InputError> {
    let mode = screen.mouse_protocol_mode();
    if mode == MouseProtocolMode::None {
        return Ok(Vec::new());
    }
    let (rows, columns) = screen.size();
    if event.column >= columns || event.row >= rows {
        return Err(InputError::MouseCoordinate);
    }
    if event
        .modifiers
        .intersects(KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META)
    {
        return Err(InputError::UnsupportedMouse);
    }
    let encoding = screen.mouse_protocol_encoding();
    let release = matches!(event.kind, MouseEventKind::Up(_));
    let mut code = match event.kind {
        MouseEventKind::Down(button) => button_code(button),
        MouseEventKind::Up(button) if mode != MouseProtocolMode::Press => {
            if encoding == MouseProtocolEncoding::Sgr {
                button_code(button)
            } else {
                RELEASE
            }
        }
        MouseEventKind::Drag(button)
            if matches!(
                mode,
                MouseProtocolMode::ButtonMotion | MouseProtocolMode::AnyMotion
            ) =>
        {
            button_code(button) + MOTION
        }
        MouseEventKind::Moved if mode == MouseProtocolMode::AnyMotion => RELEASE + MOTION,
        MouseEventKind::ScrollUp => SCROLL_UP,
        MouseEventKind::ScrollDown => SCROLL_DOWN,
        MouseEventKind::ScrollLeft => SCROLL_LEFT,
        MouseEventKind::ScrollRight => SCROLL_RIGHT,
        _ => return Ok(Vec::new()),
    };
    if mode != MouseProtocolMode::Press {
        code += u16::from(event.modifiers.contains(KeyModifiers::SHIFT)) * SHIFT
            + u16::from(event.modifiers.contains(KeyModifiers::ALT)) * ALT
            + u16::from(event.modifiers.contains(KeyModifiers::CONTROL)) * CONTROL;
    }
    if encoding == MouseProtocolEncoding::Sgr {
        let column = u32::from(event.column) + u32::from(COORDINATE_BASE);
        let row = u32::from(event.row) + u32::from(COORDINATE_BASE);
        let suffix = if release {
            RELEASE_SUFFIX
        } else {
            PRESS_SUFFIX
        };
        return Ok(format!("{SGR_PREFIX}{code};{column};{row}{suffix}").into_bytes());
    }
    let limit = if encoding == MouseProtocolEncoding::Utf8 {
        UTF8_LIMIT
    } else {
        DEFAULT_LIMIT
    };
    let column = coordinate(event.column, limit)?;
    let row = coordinate(event.row, limit)?;
    let mut output = LEGACY_PREFIX.to_vec();
    for value in [code + BYTE_OFFSET, column, row] {
        if encoding == MouseProtocolEncoding::Default {
            output.push(value as u8);
        } else {
            let value = char::from_u32(u32::from(value)).ok_or(InputError::MouseCoordinate)?;
            output.extend_from_slice(
                value
                    .encode_utf8(&mut [0; std::mem::size_of::<char>()])
                    .as_bytes(),
            );
        }
    }
    Ok(output)
}

fn coordinate(value: u16, limit: u16) -> Result<u16, InputError> {
    value
        .checked_add(COORDINATE_BASE + BYTE_OFFSET)
        .filter(|value| *value <= limit)
        .ok_or(InputError::MouseCoordinate)
}
fn button_code(button: MouseButton) -> u16 {
    match button {
        MouseButton::Left => LEFT,
        MouseButton::Middle => MIDDLE,
        MouseButton::Right => RIGHT,
    }
}
