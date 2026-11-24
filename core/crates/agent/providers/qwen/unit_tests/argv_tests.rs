use crate::agent::ObservedProcess;
use crate::qwen::argv::{self, SessionIdentity};
use crate::qwen::constants;

const SESSION: &str = "5da2e019-cf3c-4864-a102-e4201ca9afcb";

fn process(executable: &str, argv: &[&str]) -> ObservedProcess {
    ObservedProcess {
        pid: 10,
        start_time: 20,
        parent_pid: 1,
        process_group: 2,
        foreground_group: 2,
        executable: executable.into(),
        argv: argv.iter().map(|value| (*value).into()).collect(),
        environment: None,
        cwd: None,
        evidence_paths: None,
    }
}

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).into()).collect()
}

fn resume_equals() -> String {
    format!(
        "{}{}{SESSION}",
        constants::RESUME_ARGUMENT, constants::ARGUMENT_VALUE_SEPARATOR
    )
}

#[test]
fn exact_qwen_resume_flags_are_canonical() {
    let equals = resume_equals();
    for values in [
        vec![constants::SHORT_RESUME_ARGUMENT, SESSION],
        vec![constants::RESUME_ARGUMENT, SESSION],
        vec![equals.as_str()],
        vec![constants::SESSION_ID_ARGUMENT, SESSION],
    ] {
        assert_eq!(
            argv::session_identity(&args(&values)),
            SessionIdentity::Strong(SESSION.into()),
            "{values:?}"
        );
    }
    assert_eq!(
        argv::resume_session_id(&args(&[
            constants::SHORT_RESUME_ARGUMENT,
            SESSION
        ])),
        Some(SESSION.into())
    );
    assert_eq!(
        argv::resume_session_id(&args(&[constants::SESSION_ID_ARGUMENT, SESSION])),
        None
    );
}

#[test]
fn conflicting_continue_fork_and_duplicate_sessions_fail_closed() {
    for values in [
        vec![constants::CONTINUE_ARGUMENT],
        vec![constants::SHORT_CONTINUE_ARGUMENT],
        vec![constants::FORK_SESSION_ARGUMENT, constants::SHORT_RESUME_ARGUMENT, SESSION],
        vec![
            constants::SHORT_RESUME_ARGUMENT,
            SESSION,
            constants::RESUME_ARGUMENT,
            SESSION,
        ],
        vec![constants::SHORT_RESUME_ARGUMENT, "not-a-uuid"],
        vec![constants::SHORT_RESUME_ARGUMENT],
    ] {
        let identity = argv::session_identity(&args(&values));
        assert!(!identity.is_valid(), "{values:?}");
        assert_eq!(argv::resume_session_id(&args(&values)), None, "{values:?}");
    }
}

#[test]
fn node_bin_entrypoint_and_direct_executable_are_recognized() {
    let node = process(
        "/usr/bin/node",
        &[
            "node",
            "/Users/dev/.qwen/node_modules/.bin/qwen",
            constants::AUTH_TYPE_ARGUMENT,
            "openai",
            constants::RESUME_ARGUMENT,
            SESSION,
        ],
    );
    assert!(argv::matches_node_entrypoint(&node));
    assert_eq!(
        argv::command_arguments(&node).map(|values| values.len()),
        Some(4)
    );
    assert_eq!(
        argv::resume_session_id(argv::command_arguments(&node).unwrap()),
        Some(SESSION.into())
    );

    let direct = process(
        constants::EXECUTABLE,
        &[
            constants::EXECUTABLE,
            constants::SHORT_RESUME_ARGUMENT,
            SESSION,
        ],
    );
    assert!(argv::matches_node_entrypoint(&direct));
    assert_eq!(
        argv::resume_session_id(argv::command_arguments(&direct).unwrap()),
        Some(SESSION.into())
    );

    let unrelated = process("/usr/bin/node", &["node", "/tmp/other.js"]);
    assert!(!argv::matches_node_entrypoint(&unrelated));
    assert!(argv::command_arguments(&unrelated).is_none());
}
