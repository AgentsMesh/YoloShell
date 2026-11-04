#[path = "../test_support/fixtures.rs"]
mod fixtures;

mod attach;
mod attach_history;
mod capability;
mod creation;
mod creation_query;
mod creation_query_wire;
mod history;
mod input;
mod lease;
mod lease_observation;
mod lifecycle;
mod metadata;
#[path = "../test_support/creation_query.rs"]
mod query_support;
mod recovery;
mod run;
mod selector;
mod size;
mod terminal;
mod terminal_event;
