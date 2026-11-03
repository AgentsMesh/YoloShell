use super::arguments::base;
use super::constants::*;
use crate::arguments::{AgentCommand, Arguments, Command};
use clap::Parser;

#[test]
fn start_accepts_a_parent_run_selector() {
    let mut argv = base();
    argv.extend([
        START_ACTION.into(),
        PROVIDER_FLAG.into(),
        "codex".into(),
        PARENT_FLAG.into(),
        "parent-run".into(),
    ]);
    let Command::Agent(options) = Arguments::try_parse_from(argv).unwrap().command;
    let AgentCommand::Start(start) = options.command else {
        panic!("expected start")
    };
    assert_eq!(start.parent.as_deref(), Some("parent-run"));
}
