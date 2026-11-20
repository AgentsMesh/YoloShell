use super::session_id;
use crate::agent::ObservedProcess;
use crate::codex::{argv, constants, test_support};
use std::path::PathBuf;

const NODE_ENTRYPOINT: &str = "/opt/node_modules/@openai/codex/bin/codex.js";
const PROJECT_CWD: &str = "/project";
const OTHER_SESSION_ID: &str = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
const COMPACT_SESSION_ID: &str = "550e8400e29b41d4a716446655440000";
const RESUME_PROMPT: &str = "continue";
const RELATIVE_CWD: &str = "project";

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

fn direct(arguments: &[&str]) -> ObservedProcess {
    let mut argv = vec![constants::EXECUTABLE];
    argv.extend_from_slice(arguments);
    process(constants::EXECUTABLE, &argv)
}

#[test]
fn accepts_only_complete_yolo_resume_dialects() {
    let expected = Some(test_support::SESSION_ID.to_owned());
    assert_eq!(
        session_id(&direct(&[
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
        ])),
        expected
    );
    assert_eq!(
        session_id(&direct(&[
            constants::RESUME_ARGUMENT,
            constants::CWD_ARGUMENT,
            PROJECT_CWD,
            test_support::SESSION_ID,
        ])),
        expected
    );
    assert_eq!(
        session_id(&process(
            constants::NODE_EXECUTABLE,
            &[
                constants::NODE_EXECUTABLE,
                NODE_ENTRYPOINT,
                constants::RESUME_ARGUMENT,
                constants::CWD_ARGUMENT,
                PROJECT_CWD,
                test_support::SESSION_ID,
            ],
        )),
        expected
    );
}

#[test]
fn rejects_selector_ambiguity_and_every_trailing_token() {
    for arguments in [
        vec![
            constants::RESUME_ARGUMENT,
            constants::LAST_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
            constants::LAST_ARGUMENT,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
            OTHER_SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
            constants::RESUME_ARGUMENT,
            OTHER_SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
            constants::FORK_ARGUMENT,
            OTHER_SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            test_support::SESSION_ID,
            RESUME_PROMPT,
        ],
    ] {
        assert_eq!(session_id(&direct(&arguments)), None);
    }
    assert_eq!(session_id(&process(constants::EXECUTABLE, &[])), None);
}

#[test]
fn rejects_options_not_emitted_by_yolo() {
    for arguments in [
        vec![
            constants::RESUME_ARGUMENT,
            constants::TEST_UNKNOWN_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            constants::TEST_REMOTE_ENDPOINT_ARGUMENT,
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            constants::VARIABLE_VALUE_ARGUMENTS[0],
            test_support::SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            constants::OPTION_TERMINATOR,
            test_support::SESSION_ID,
        ],
        vec![constants::RESUME_ARGUMENT, constants::CWD_ARGUMENT],
        vec![
            constants::RESUME_ARGUMENT,
            constants::CWD_ARGUMENT,
            PROJECT_CWD,
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
            RELATIVE_CWD,
            test_support::SESSION_ID,
        ],
    ] {
        assert_eq!(session_id(&direct(&arguments)), None);
    }
}

#[test]
fn requires_canonical_uuid_and_preserves_loose_association_parsing() {
    let uppercase = test_support::SESSION_ID.to_uppercase();
    for candidate in [COMPACT_SESSION_ID, uppercase.as_str()] {
        let process = direct(&[constants::RESUME_ARGUMENT, candidate]);
        assert!(argv::session_identity(&process).into_session_id().is_some());
        assert_eq!(session_id(&process), None);
    }
    let with_prompt = direct(&[
        constants::RESUME_ARGUMENT,
        test_support::SESSION_ID,
        RESUME_PROMPT,
    ]);
    assert!(
        argv::session_identity(&with_prompt)
            .into_session_id()
            .is_some()
    );
    assert_eq!(session_id(&with_prompt), None);
}
