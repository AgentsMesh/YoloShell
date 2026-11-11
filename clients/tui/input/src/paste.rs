use crate::{InputError, constants::*};
use yolo_hub_terminal_protocol::MAX_INPUT_BYTES;

pub fn encode_paste(text: &str, bracketed: bool) -> Result<Vec<u8>, InputError> {
    // An embedded bracket-end sequence could turn pasted text into terminal commands.
    if text
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(InputError::UnsafePaste);
    }
    let overhead = if bracketed {
        PASTE_START.len() + PASTE_END.len()
    } else {
        0
    };
    if text.len() > MAX_INPUT_BYTES.saturating_sub(overhead) {
        return Err(InputError::PayloadTooLarge);
    }
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let mut output = Vec::with_capacity(text.len() + overhead);
    if bracketed {
        output.extend_from_slice(PASTE_START.as_bytes());
    }
    output.extend_from_slice(text.as_bytes());
    if bracketed {
        output.extend_from_slice(PASTE_END.as_bytes());
    }
    Ok(output)
}
