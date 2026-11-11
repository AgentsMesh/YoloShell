use super::argv::{SessionIdentity, resume_session_id, session_identity};
use super::constants;

const SESSION_ID: &str = "0b81c1f2-6d61-48af-8703-ac0b453c23ab";
const OTHER_SESSION_ID: &str = "1559fcf1-a6d0-44b8-90a7-6ce2cfcb199f";
const INVALID_SESSION_ID: &str = "not-a-uuid";

fn parse(arguments: &[&str]) -> SessionIdentity {
    session_identity(
        &arguments
            .iter()
            .map(|argument| (*argument).to_owned())
            .collect::<Vec<_>>(),
    )
}

#[test]
fn strict_identity_flags_accept_separate_and_equals_forms() {
    let expected = SessionIdentity::Strong(SESSION_ID.into());
    assert_eq!(parse(&[constants::RESUME_ARGUMENT, SESSION_ID]), expected);
    assert_eq!(
        parse(&[constants::SHORT_RESUME_ARGUMENT, SESSION_ID]),
        expected
    );
    let equals = format!(
        "{}{}{}",
        constants::SESSION_ID_ARGUMENT,
        constants::ARGUMENT_VALUE_SEPARATOR,
        SESSION_ID
    );
    assert_eq!(parse(&[constants::EXECUTABLE, &equals]), expected);
    assert_eq!(
        parse(&[constants::EXECUTABLE, constants::CONTINUE_ARGUMENT]),
        SessionIdentity::Absent
    );
    assert_eq!(
        parse(&[constants::EXECUTABLE, constants::SHORT_CONTINUE_ARGUMENT,]),
        SessionIdentity::Absent
    );
    assert_eq!(
        parse(&[
            constants::EXECUTABLE,
            constants::ARGUMENT_TERMINATOR,
            constants::RESUME_ARGUMENT,
            SESSION_ID,
        ]),
        SessionIdentity::Absent
    );
}

#[test]
fn malformed_duplicate_and_noncanonical_identities_fail_closed() {
    let uppercase = SESSION_ID.to_uppercase();
    for arguments in [
        vec![constants::RESUME_ARGUMENT],
        vec![constants::SHORT_RESUME_ARGUMENT],
        vec![constants::SESSION_ID_ARGUMENT, INVALID_SESSION_ID],
        vec![constants::RESUME_ARGUMENT, &uppercase],
        vec![
            constants::RESUME_ARGUMENT,
            SESSION_ID,
            constants::SESSION_ID_ARGUMENT,
            OTHER_SESSION_ID,
        ],
        vec![
            constants::SHORT_RESUME_ARGUMENT,
            SESSION_ID,
            constants::RESUME_ARGUMENT,
            OTHER_SESSION_ID,
        ],
    ] {
        assert_eq!(parse(&arguments), SessionIdentity::Invalid);
    }
}

#[test]
fn exact_resume_identity_rejects_fresh_continue_session_id_and_fork_modes() {
    let resume_equals = format!(
        "{}{}{}",
        constants::RESUME_ARGUMENT,
        constants::ARGUMENT_VALUE_SEPARATOR,
        SESSION_ID
    );
    for arguments in [
        vec![constants::RESUME_ARGUMENT, SESSION_ID],
        vec![constants::SHORT_RESUME_ARGUMENT, SESSION_ID],
        vec![resume_equals.as_str()],
        vec![
            constants::RESUME_ARGUMENT,
            SESSION_ID,
            constants::ARGUMENT_TERMINATOR,
            constants::FORK_SESSION_ARGUMENT,
        ],
    ] {
        assert_eq!(
            resume_session_id(&arguments.into_iter().map(str::to_owned).collect::<Vec<_>>()),
            Some(SESSION_ID.to_owned())
        );
    }

    for arguments in [
        vec![constants::SESSION_ID_ARGUMENT, SESSION_ID],
        vec![constants::CONTINUE_ARGUMENT],
        vec![constants::SHORT_CONTINUE_ARGUMENT],
        vec![constants::RESUME_ARGUMENT],
        vec![
            constants::RESUME_ARGUMENT,
            SESSION_ID,
            constants::FORK_SESSION_ARGUMENT,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            SESSION_ID,
            constants::SESSION_ID_ARGUMENT,
            SESSION_ID,
        ],
        vec![
            constants::RESUME_ARGUMENT,
            SESSION_ID,
            constants::RESUME_ARGUMENT,
            SESSION_ID,
        ],
    ] {
        assert_eq!(
            resume_session_id(&arguments.into_iter().map(str::to_owned).collect::<Vec<_>>()),
            None
        );
    }
}
