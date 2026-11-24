use crate::constants::*;

pub fn rejects_while_alive(mode: &str) -> bool {
    matches!(
        mode,
        BAD_ID | BAD_ORIGIN | BAD_SCHEMA | OMIT_ROOT | DUPLICATE | OVERSIZED | CLOSED_CONTROL
    )
}

pub fn requires_interrupted_cleanup(mode: &str) -> bool {
    rejects_while_alive(mode) || mode == HANG
}
