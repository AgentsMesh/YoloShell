use super::*;

const VARIABLE: &str = "TOOL";
const OTHER_VARIABLE: &str = "UNKNOWN";
const CLEAN_EXECUTABLE: &str = "echo";

#[test]
fn malformed_assignments_do_not_mutate_previously_recorded_values() {
    let mut state = State::default();
    assert!(state.record_assignment(&format!("{VARIABLE}={CARGO_EXECUTABLE}")));
    for word in [
        "=value",
        "invalid-name=value",
        "invalid.name=value",
        "not_assignment",
    ] {
        assert!(!state.record_assignment(word), "{word}");
        assert_eq!(state.variables[VARIABLE], CARGO_EXECUTABLE);
    }
}

#[test]
fn literal_dollars_incomplete_expansions_and_empty_values_remain_distinct() {
    let mut state = State::default();
    state
        .variables
        .insert(VARIABLE.into(), CARGO_EXECUTABLE.into());
    for (input, expected) in [
        (format!(r"\${VARIABLE}"), format!("${VARIABLE}")),
        (format!("${{{VARIABLE}}}"), CARGO_EXECUTABLE.into()),
        (format!("${VARIABLE}"), CARGO_EXECUTABLE.into()),
        (
            format!("${{{OTHER_VARIABLE}}}"),
            format!("${OTHER_VARIABLE}"),
        ),
        (format!("${{{VARIABLE}"), format!("${{{VARIABLE}")),
        ("$".into(), "$".into()),
        ("$?".into(), "$?".into()),
        ("trailing\\".into(), "trailing\\".into()),
    ] {
        assert_eq!(state.expand(&input), expected, "{input}");
    }
    state.variables.insert(VARIABLE.into(), String::new());
    assert_eq!(state.expand(&format!("${VARIABLE}")), String::new());
    assert_eq!(
        state.expand(&format!("${{{VARIABLE}}}")),
        format!("${VARIABLE}")
    );
}

#[test]
fn defaults_expand_references_without_replacing_nonempty_assignments() {
    let mut state = State::default();
    state
        .variables
        .insert(VARIABLE.into(), CARGO_EXECUTABLE.into());
    for operator in [":-", ":=", "-", "="] {
        assert_eq!(
            state.expand(&format!("${{{OTHER_VARIABLE}{operator}${VARIABLE}}}")),
            CARGO_EXECUTABLE
        );
        assert_eq!(
            state.expand(&format!("${{{VARIABLE}{operator}{CLEAN_EXECUTABLE}}}")),
            CARGO_EXECUTABLE
        );
    }
    assert_eq!(expanded_command_basename(" \t"), "");
    assert_eq!(default_expression(VARIABLE), (VARIABLE, None));
}

#[test]
fn nested_shell_and_env_assignments_do_not_escape_their_execution_scope() {
    let mut state = State::default();
    for words in [
        vec![
            "sh".into(),
            "-c".into(),
            format!("{VARIABLE}={CARGO_EXECUTABLE}; echo ready"),
        ],
        vec![
            "env".into(),
            "-S".into(),
            format!("{VARIABLE}={CARGO_EXECUTABLE} echo ready"),
        ],
    ] {
        assert!(!state.executes(&words));
        assert!(!state.variables.contains_key(VARIABLE));
    }
    assert!(!state.executes(&[]));
    assert!(!state.executes(&["sh".into(), "-c".into()]));
}
