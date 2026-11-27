//! Durable UI intent for a single terminal incarnation, independent of transport.
//! This policy requests ordinary acquisition only. The adapter validates grants,
//! owns writer capabilities, and never replays input when control is recovered.

mod acquisition;
mod constants;
mod controller;
mod model;

pub use constants::{ATTEMPT_TIMEOUT, RETRY_INTERVAL};
pub use controller::Controller;
pub use model::{Attempt, Failure, Intent, Presentation};
