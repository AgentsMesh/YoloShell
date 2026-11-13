pub const MAX_PROMPT_BYTES: usize = 65_536;
pub(crate) const INVALID_PROMPT: &str =
    "prompt must contain bounded text without terminal controls";
pub(crate) const LINE_FEED: char = '\n';
pub(crate) const HORIZONTAL_TAB: char = '\t';
