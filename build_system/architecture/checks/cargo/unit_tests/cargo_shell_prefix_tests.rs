use crate::cargo_shell_prefixes::{Prefix, command_prefix, skip_controls};
use crate::policy_constants::*;

#[path = "../test_support/cargo_shell_constants.rs"]
mod constants;
use constants::{EXEC_LOGIN_OPTION, LONG_COMMAND_OPTION, TIME_PORTABLE_OPTION};

const PROGRAM: &str = CARGO_EXECUTABLE;
const ARGUMENT: &str = "opaque";

fn words(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn cursor(prefix: &str, values: &[&str], expected: usize) {
    let Prefix::Continue(actual) = command_prefix(prefix, &words(values), 0) else {
        panic!("expected executable prefix: {prefix} {values:?}");
    };
    assert_eq!(actual, expected, "{prefix} {values:?}");
}

#[test]
fn command_query_and_option_end_do_not_consume_the_executable() {
    for option in COMMAND_QUERY_OPTIONS {
        assert!(matches!(
            command_prefix(POLICY_COMMAND, &words(&[option, PROGRAM]), 0),
            Prefix::NonExecuting
        ));
    }
    cursor(POLICY_COMMAND, &[OPTION_END, PROGRAM], 1);
    cursor(POLICY_COMMAND, &["-p", PROGRAM], 1);
    cursor(POLICY_COMMAND, &[], 0);
    cursor(PROGRAM, &[ARGUMENT], 0);
}

#[test]
fn env_split_strings_support_attached_separate_and_clustered_options() {
    for option in ENV_SPLIT_OPTIONS {
        let Prefix::Nested(actual) = command_prefix(POLICY_ENV, &words(&[option, PROGRAM]), 0)
        else {
            panic!("{option}");
        };
        assert_eq!(actual, PROGRAM);
        assert!(matches!(
            command_prefix(POLICY_ENV, &words(&[option]), 0),
            Prefix::NonExecuting
        ));
    }
    for option in [
        format!("{ENV_SPLIT_PREFIX}{PROGRAM}"),
        format!("-ivS{PROGRAM}"),
    ] {
        let Prefix::Nested(actual) = command_prefix(POLICY_ENV, &[option], 0) else {
            panic!("split prefix");
        };
        assert_eq!(actual, PROGRAM);
    }
    let Prefix::Nested(actual) = command_prefix(POLICY_EVAL, &words(&[PROGRAM, ARGUMENT]), 0)
    else {
        panic!("eval prefix");
    };
    assert_eq!(actual, format!("{PROGRAM} {ARGUMENT}"));
}

#[test]
fn env_value_options_flag_clusters_and_end_are_bounded() {
    for option in ENV_VALUE_OPTIONS {
        cursor(POLICY_ENV, &[option, ARGUMENT, PROGRAM], 2);
    }
    for option in ["-Cdirectory", "-Ppath", "-aargv", "-uvariable", "-0iv"] {
        cursor(POLICY_ENV, &[option, PROGRAM], 1);
    }
    for option in NON_EXECUTING_OPTIONS
        .iter()
        .copied()
        .chain(["-invalid", "-?"])
    {
        assert!(matches!(
            command_prefix(POLICY_ENV, &words(&[option, PROGRAM]), 0),
            Prefix::NonExecuting
        ));
    }
    cursor(POLICY_ENV, &[OPTION_END, PROGRAM], 1);
    cursor(POLICY_ENV, &[XCRUN_VALUE_OPTIONS[0], PROGRAM], 1);
    cursor(POLICY_ENV, &["-", PROGRAM], 1);
    cursor(POLICY_ENV, &[PROGRAM], 0);
    cursor(POLICY_ENV, &[], 0);
}

#[test]
fn launcher_options_obey_query_value_and_option_end_contracts() {
    for (prefix, values, queries) in [
        (POLICY_NICE, NICE_VALUE_OPTIONS, NON_EXECUTING_OPTIONS),
        (POLICY_NOHUP, &[] as &[&str], NON_EXECUTING_OPTIONS),
        (POLICY_SUDO, SUDO_VALUE_OPTIONS, SUDO_QUERY_OPTIONS),
        (POLICY_XCRUN, XCRUN_VALUE_OPTIONS, XCRUN_QUERY_OPTIONS),
    ] {
        for option in values {
            cursor(prefix, &[option, ARGUMENT, PROGRAM], 2);
        }
        for option in queries {
            assert!(matches!(
                command_prefix(prefix, &words(&[option, PROGRAM]), 0),
                Prefix::NonExecuting
            ));
        }
        cursor(prefix, &[OPTION_END, PROGRAM], 1);
        cursor(prefix, &["-unknown", PROGRAM], 1);
        cursor(prefix, &[], 0);
    }
    for option in EXEC_VALUE_OPTIONS {
        cursor(POLICY_EXEC, &[option, ARGUMENT, PROGRAM], 2);
    }
    cursor(POLICY_EXEC, &[OPTION_END, PROGRAM], 1);
    cursor(POLICY_EXEC, &[], 0);
}

#[test]
fn controls_and_timing_options_preserve_the_execution_cursor() {
    let mut values = words(CONTROL_WORDS);
    assert_eq!(skip_controls(&values, 0), values.len());
    values.extend(words(&[
        TIME_COMMAND,
        TIME_VALUE_OPTIONS[0],
        ARGUMENT,
        OPTION_END,
        PROGRAM,
    ]));
    assert_eq!(skip_controls(&values, 0), values.len() - 1);
    assert_eq!(skip_controls(&words(&[TIME_COMMAND, PROGRAM]), 0), 1);
}

#[test]
fn flag_only_prefixes_cannot_hide_the_following_cargo_executable() {
    cursor(POLICY_EXEC, &[EXEC_LOGIN_OPTION, PROGRAM], 1);
    assert_eq!(
        skip_controls(&words(&[TIME_COMMAND, TIME_PORTABLE_OPTION, PROGRAM]), 0),
        2
    );
    for prefix in [
        format!("{POLICY_EXEC} {EXEC_LOGIN_OPTION}"),
        format!("{TIME_COMMAND} {TIME_PORTABLE_OPTION}"),
    ] {
        assert_eq!(
            crate::cargo_shell::execution_lines(&format!("{prefix} {PROGRAM}")),
            [1]
        );
    }
}

#[test]
fn shell_command_clusters_require_a_single_dash_and_ascii_letter_flags() {
    for (option, expected) in [
        ("c", false),
        (LONG_COMMAND_OPTION, false),
        ("-", false),
        ("-c2", false),
        ("-c", true),
        ("-lic", true),
        ("-l", false),
    ] {
        assert_eq!(
            crate::cargo_shell_words::shell_command_option(option),
            expected,
            "{option}"
        );
    }
}
