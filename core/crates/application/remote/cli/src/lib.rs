//! Argument parsing and dispatch for the remote process role.
mod cli;
mod conversion;
mod dispatch;
mod entry;
mod entry_session;
mod error_constants;
mod options;
mod output;

use cli::{Cli, Command, CommandOptions, FileOptions, ServerOptions};
use dispatch::run;
pub use entry::run_from;

#[cfg(test)]
#[path = "../test_support/constants.rs"]
mod fixture_constants;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/tests.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests.rs"]
mod unit_tests;
