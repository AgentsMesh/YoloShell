mod constants;
#[path = "../test_support/view.rs"]
mod view;

mod candidate_activity;
#[path = "../test_support/native_control.rs"]
mod control_support;
mod directory;
mod fixtures;
mod lifecycle;
mod lost_runtime;
mod native_control;
mod native_control_retention;
mod projection_policy;
mod prompt_readiness;
mod record_migration;
mod retention;
