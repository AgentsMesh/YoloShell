#[path = "constants.rs"]
mod constants;
#[path = "framing.rs"]
mod framing;
pub mod native_outbox;
#[path = "outbox.rs"]
#[doc(hidden)]
pub mod outbox;

pub use constants::*;
pub use framing::{read_frame, write_frame};
pub use yolo_claude_hook_contract::*;

#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[doc(hidden)]
pub mod test_outbox_error {
    pub use super::constants::outbox_error::INVOCATION_CAPACITY;
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod tests;
