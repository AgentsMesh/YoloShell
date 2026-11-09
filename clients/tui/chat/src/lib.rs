//! A client-local conversation view. No terminal ownership, provider files or network access.
mod composer;
mod constants;
mod history;
mod model;
mod query;
mod readiness;
mod render;

pub use composer::{Composer, DraftError, SendPhase};
pub use constants::{DRAFT_LOCKED, DRAFT_UNAVAILABLE, MAX_HISTORY_PAGES, MAX_RENDER_ROWS};
pub use model::{Chat, Mode};
pub use query::{Completion, Query, QueryError, QueryKind, ReadKind};
pub use readiness::InputReadiness;
pub use render::{Viewport, render};
