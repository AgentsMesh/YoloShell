use crate::{Selection, SelectionError, constants::*};
use base64::{Engine, engine::general_purpose::STANDARD};

/// An outer-terminal command constructed only from a completed local selection.
/// No provider escape sequence or external string can construct this value.
#[derive(Debug, Eq, PartialEq)]
pub struct ClipboardCommand {
    text: String,
    osc: Vec<u8>,
}

impl ClipboardCommand {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.osc
    }
}

impl Selection {
    /// The UI invokes this only for its explicit copy gesture, never PTY output.
    pub fn clipboard(&self) -> Result<ClipboardCommand, SelectionError> {
        let text = self.text()?;
        let encoded = STANDARD.encode(&text);
        let mut command =
            Vec::with_capacity(CLIPBOARD_PREFIX.len() + encoded.len() + CLIPBOARD_END.len());
        command.extend_from_slice(CLIPBOARD_PREFIX);
        command.extend_from_slice(encoded.as_bytes());
        command.extend_from_slice(CLIPBOARD_END);
        Ok(ClipboardCommand { text, osc: command })
    }
}
