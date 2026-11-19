use super::super::constants::TEST_STATUS_ARGUMENT;
use super::super::constants::{
    MAX_COMMAND_ARGUMENTS, MAX_COMMAND_OUTPUT_BYTES, MAX_COMMAND_TIMEOUT,
};
use super::super::*;
use std::time::Duration;

const COMMAND_TIMEOUT_OVERAGE: Duration = Duration::from_secs(1);
const COMMAND_OUTPUT_LIMIT_OVERAGE: usize = 1;
const INVALID_COMMAND_OUTPUT_LIMIT: usize = 0;
const MINIMUM_COMMAND_OUTPUT_LIMIT: usize = 1;

#[test]
fn command_policy_rejects_unlisted_and_unbounded_commands() {
    let policy = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    assert!(matches!(
        policy.authorize(CommandRequest::new("/bin/sh", vec![])),
        Err(RemoteError::CommandDenied(_))
    ));
    assert!(matches!(
        policy.authorize(
            CommandRequest::new("/usr/bin/git", vec![])
                .with_limits(Duration::ZERO, MINIMUM_COMMAND_OUTPUT_LIMIT)
        ),
        Err(RemoteError::InvalidCommand(_))
    ));
}

#[test]
fn every_command_bound_is_enforced_independently() {
    assert!(CommandPolicy::new(Vec::<String>::new()).is_err());
    assert!(CommandPolicy::new(["/usr/bin/git".into(), "relative".into()]).is_err());
    assert!(CommandPolicy::new([String::new()]).is_err());
    assert!(CommandPolicy::new(["/usr/bin/bad\0program".into()]).is_err());
    let policy = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    let invalid = [
        CommandRequest::new("relative", vec![]),
        CommandRequest::new("/usr/bin/git", vec![String::new()]),
        CommandRequest::new("/usr/bin/git", vec!["nul\0value".into()]),
        CommandRequest::new("/usr/bin/git", vec![])
            .with_limits(MAX_COMMAND_TIMEOUT, INVALID_COMMAND_OUTPUT_LIMIT),
        CommandRequest::new("/usr/bin/git", vec![]).with_limits(
            MAX_COMMAND_TIMEOUT,
            MAX_COMMAND_OUTPUT_BYTES + COMMAND_OUTPUT_LIMIT_OVERAGE,
        ),
        CommandRequest::new("/usr/bin/git", vec![]).with_limits(
            MAX_COMMAND_TIMEOUT + COMMAND_TIMEOUT_OVERAGE,
            MINIMUM_COMMAND_OUTPUT_LIMIT,
        ),
    ];
    for request in invalid {
        assert!(matches!(
            policy.authorize(request),
            Err(RemoteError::InvalidCommand(_))
        ));
    }
    let too_many = vec![TEST_STATUS_ARGUMENT.into(); MAX_COMMAND_ARGUMENTS + 1];
    assert!(
        policy
            .authorize(CommandRequest::new("/usr/bin/git", too_many))
            .is_err()
    );
}

#[test]
fn backend_output_cannot_exceed_the_authorized_limit() {
    let policy = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    let command = policy
        .authorize(
            CommandRequest::new("/usr/bin/git", vec![])
                .with_limits(MAX_COMMAND_TIMEOUT, MINIMUM_COMMAND_OUTPUT_LIMIT),
        )
        .unwrap();
    let output = CommandOutput {
        stdout: vec![0],
        stderr: vec![0],
        exit_status: Some(0),
        timed_out: false,
        truncated: false,
    };
    assert!(matches!(
        output.validate(&command),
        Err(RemoteError::LimitExceeded)
    ));
}

#[test]
fn command_policy_accepts_safe_bounded_arguments() {
    let policy = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    let request = CommandRequest::new("/usr/bin/git", vec![TEST_STATUS_ARGUMENT.into()])
        .with_cwd("/Users/remote/work");
    let command = policy.authorize(request).unwrap();
    assert_eq!(command.request().args(), &[TEST_STATUS_ARGUMENT]);
}

#[test]
fn command_policy_rejects_relative_or_noncanonical_paths() {
    for program in [
        "git",
        "/",
        "/usr/../bin/git",
        "/usr//bin/git",
        "/usr/./bin/git",
    ] {
        assert!(CommandPolicy::new([program.into()]).is_err());
    }
    let policy = CommandPolicy::new(["/usr/bin/git".into()]).unwrap();
    for cwd in ["relative", "/", "/srv/../tmp", "/srv//tmp", "/srv/./tmp"] {
        assert!(
            policy
                .authorize(CommandRequest::new("/usr/bin/git", vec![]).with_cwd(cwd))
                .is_err()
        );
    }
}
