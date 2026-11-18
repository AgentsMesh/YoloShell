use crate::{MAX_TITLE_BYTES, constants::*};
use yolo_terminal_metadata_protocol::valid_text;

/// Read the current snapshot's bounded title prefix without rendering its screen.
/// No search or historical metadata replay is performed.
pub fn title_from_snapshot(data: &[u8]) -> Option<String> {
    let data = data.strip_prefix(TITLE_PREFIX)?;
    let bounded = &data[..data.len().min(MAX_TITLE_BYTES + STRING_TERMINATOR.len())];
    let end = bounded
        .windows(STRING_TERMINATOR.len())
        .position(|bytes| bytes == STRING_TERMINATOR)?;
    let title = std::str::from_utf8(&bounded[..end]).ok()?;
    valid_text(title, MAX_TITLE_BYTES).then(|| title.to_owned())
}
