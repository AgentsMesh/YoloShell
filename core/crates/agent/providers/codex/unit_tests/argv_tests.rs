use super::{SessionIdentity, session_identity};
use crate::agent::ObservedProcess;
use crate::codex::{constants, test_support};
use std::path::PathBuf;

const ABSOLUTE_CODEX: &str = "/usr/local/bin/codex";
const NODE_ENTRYPOINT: &str = "/opt/node_modules/@openai/codex/bin/codex.js";
const WRONG_ENTRYPOINT: &str = "/opt/node_modules/other/bin/codex.js";
const INVALID_SESSION_ID: &str = "not-a-uuid";
const COMPACT_SESSION_ID: &str = "550e8400e29b41d4a716446655440000";
const MODEL_ARGUMENT: &str = "-m";
const PROJECT_CWD: &str = "/project";
const SESSION_NAME: &str = "named-session";
const RESUME_PROMPT: &str = "continue";

fn process(executable: &str, argv: &[&str]) -> ObservedProcess {
    ObservedProcess {
        pid: test_support::PROCESS_PID,
        start_time: test_support::PROCESS_START_TIME,
        parent_pid: crate::agent::constants::MIN_PROCESS_PID,
        process_group: test_support::PROCESS_PID.try_into().unwrap(),
        foreground_group: test_support::PROCESS_PID.try_into().unwrap(),
        executable: executable.to_owned(),
        argv: argv.iter().map(|value| (*value).to_owned()).collect(),
        environment: None,
        cwd: Some(PathBuf::from(PROJECT_CWD)),
        evidence_paths: None,
    }
}

fn direct(arguments: &[&str]) -> SessionIdentity {
    let mut argv = vec![constants::EXECUTABLE];
    argv.extend_from_slice(arguments);
    session_identity(&process(constants::EXECUTABLE, &argv))
}

#[test]
fn direct_absolute_and_node_launches_are_classified() {
    assert_eq!(direct(&[]), SessionIdentity::Fresh);
    assert_eq!(direct(&[constants::YOLO_ARGUMENT]), SessionIdentity::Fresh);
    assert_eq!(
        session_identity(&process(ABSOLUTE_CODEX, &[ABSOLUTE_CODEX])),
        SessionIdentity::Fresh
    );
    assert_eq!(
        session_identity(&process(
            constants::NODE_EXECUTABLE,
            &[constants::NODE_EXECUTABLE, NODE_ENTRYPOINT]
        )),
        SessionIdentity::Fresh
    );
}

#[test]
fn current_native_and_node_resume_forms_return_a_canonical_identity() {
    let expected = SessionIdentity::Resume(test_support::SESSION_ID.to_owned());
    for arguments in [
        vec![constants::RESUME_ARGUMENT, test_support::SESSION_ID],
        vec![
            constants::YOLO_ARGUMENT,
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            constants::YOLO_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
            constants::YOLO_ARGUMENT,
        ],
        vec![
            constants::CWD_ARGUMENT,
            PROJECT_CWD,
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            constants::CWD_ARGUMENT,
            PROJECT_CWD,
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
            RESUME_PROMPT,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            constants::OPTION_TERMINATOR,
            test_support::SESSION_ID,
        ],
    ] {
        assert_eq!(direct(&arguments), expected);
    }
    assert_eq!(
        session_identity(&process(
            constants::NODE_EXECUTABLE,
            &[
                constants::NODE_EXECUTABLE,
                NODE_ENTRYPOINT,
                constants::RESUME_ARGUMENT,
                constants::YOLO_ARGUMENT,
                test_support::SESSION_ID,
            ],
        )),
        expected
    );
}

#[test]
fn picker_names_and_noncanonical_ids_require_process_association() {
    let uppercase = test_support::SESSION_ID.to_uppercase();
    for arguments in [
        vec![constants::RESUME_ARGUMENT],
        vec![constants::RESUME_ARGUMENT, INVALID_SESSION_ID],
        vec![constants::RESUME_ARGUMENT, constants::YOLO_ARGUMENT],
        vec![constants::RESUME_ARGUMENT, constants::LAST_ARGUMENT],
        vec![constants::RESUME_ARGUMENT, SESSION_NAME],
    ] {
        assert_eq!(direct(&arguments), SessionIdentity::AssociatedOnly);
    }
    let expected = SessionIdentity::Resume(test_support::SESSION_ID.to_owned());
    for arguments in [
        vec![constants::RESUME_ARGUMENT, COMPACT_SESSION_ID],
        vec![constants::RESUME_ARGUMENT, &uppercase],
    ] {
        assert_eq!(direct(&arguments), expected);
    }
    for arguments in [
        vec![
            MODEL_ARGUMENT,
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![constants::OPTION_TERMINATOR, constants::RESUME_ARGUMENT],
    ] {
        assert_eq!(direct(&arguments), SessionIdentity::Fresh);
    }
}

#[test]
fn remote_and_malformed_process_shapes_fail_closed() {
    for arguments in [
        vec![
            constants::RESUME_ARGUMENT,
            constants::TEST_REMOTE_ENDPOINT_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![constants::RESUME_ARGUMENT, constants::CWD_ARGUMENT],
        vec![constants::RESUME_ARGUMENT, constants::TEST_UNKNOWN_ARGUMENT],
    ] {
        assert_eq!(direct(&arguments), SessionIdentity::Invalid);
    }
    assert_eq!(
        session_identity(&process(constants::EXECUTABLE, &[])),
        SessionIdentity::Invalid
    );
    assert_eq!(
        session_identity(&process(
            constants::EXECUTABLE,
            &[constants::NODE_EXECUTABLE]
        )),
        SessionIdentity::Invalid
    );
    assert_eq!(
        session_identity(&process(
            constants::NODE_EXECUTABLE,
            &[constants::NODE_EXECUTABLE, WRONG_ENTRYPOINT]
        )),
        SessionIdentity::Invalid
    );
}
