#[path = "../test_support/constants.rs"]
pub(crate) mod constants;
mod corruption;
#[path = "../test_support/fixtures.rs"]
mod fixtures;
mod initialization;
mod lifecycle;
mod namespace;
mod record;
mod revision;
mod schema_rejection;
#[path = "../test_support/records.rs"]
mod support;
