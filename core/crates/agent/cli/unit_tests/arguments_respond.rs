use super::{arguments::base, constants::*};
use crate::arguments::{AgentCommand, Arguments, Command};
use crate::constants::STATE_RESPONDABLE;
use clap::Parser;

#[test]
fn respond_parses_current_and_selector_modes_and_respondable_wait() {
    let mut current = base();
    current.extend([RESPOND_ACTION.into(), CURRENT_FLAG.into(), "allow".into()]);
    let Command::Agent(options) = Arguments::try_parse_from(current).unwrap().command;
    let AgentCommand::Respond {
        selector,
        current,
        choice,
        ..
    } = options.command
    else {
        panic!("expected respond")
    };
    assert!(selector.is_none() && current && choice == "allow");

    let mut explicit = base();
    explicit.extend([RESPOND_ACTION.into(), "term-1".into(), "deny".into()]);
    let Command::Agent(options) = Arguments::try_parse_from(explicit).unwrap().command;
    let AgentCommand::Respond {
        selector,
        current,
        choice,
        ..
    } = options.command
    else {
        panic!("expected respond")
    };
    assert_eq!(selector.as_deref(), Some("term-1"));
    assert!(!current);
    assert_eq!(choice, "deny");

    let mut wait = base();
    wait.extend([
        WAIT_ACTION.into(),
        CURRENT_FLAG.into(),
        UNTIL_FLAG.into(),
        STATE_RESPONDABLE.into(),
    ]);
    let Command::Agent(options) = Arguments::try_parse_from(wait).unwrap().command;
    let AgentCommand::Wait(args) = options.command else {
        panic!("expected wait")
    };
    assert_eq!(args.until, vec![STATE_RESPONDABLE.to_owned()]);
}
