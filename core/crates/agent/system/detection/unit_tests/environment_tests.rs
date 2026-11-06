use super::constants::{
    ENVIRONMENT_ASSIGNMENT, ENVIRONMENT_NUL, MAX_PROVIDER_ENVIRONMENT_BYTES,
    MAX_PROVIDER_ENVIRONMENT_VARIABLES, RESERVED_ENVIRONMENT_PREFIX,
};
use super::{LaunchEnvironment, validate_launch_environment};

const VALID_KEY: &str = "PROVIDER_HOME";
const VALID_VALUE: &str = "/provider/home";
const KEY_PREFIX: &str = "KEY";
const KEY_FILL: char = 'K';

fn environment(key: String, value: String) -> LaunchEnvironment {
    [(key, value)].into()
}

#[test]
fn provider_environment_accepts_bounded_non_reserved_values() {
    let environment = environment(VALID_KEY.to_owned(), VALID_VALUE.to_owned());
    validate_launch_environment(&environment).unwrap();
}

#[test]
fn provider_environment_rejects_invalid_keys_and_values() {
    let cases = [
        environment(String::new(), String::new()),
        environment(
            format!("{KEY_PREFIX}{ENVIRONMENT_ASSIGNMENT}"),
            String::new(),
        ),
        environment(format!("{KEY_PREFIX}{ENVIRONMENT_NUL}"), String::new()),
        environment(
            format!("{RESERVED_ENVIRONMENT_PREFIX}{KEY_PREFIX}"),
            String::new(),
        ),
        environment(
            VALID_KEY.to_owned(),
            format!("{VALID_VALUE}{ENVIRONMENT_NUL}"),
        ),
    ];
    for environment in cases {
        assert!(validate_launch_environment(&environment).is_err());
    }
}

#[test]
fn provider_environment_enforces_count_and_byte_budgets() {
    let too_many: LaunchEnvironment = (0..=MAX_PROVIDER_ENVIRONMENT_VARIABLES)
        .map(|index| (format!("{KEY_PREFIX}{index}"), String::new()))
        .collect();
    assert!(validate_launch_environment(&too_many).is_err());

    let oversized = environment(
        std::iter::repeat_n(KEY_FILL, MAX_PROVIDER_ENVIRONMENT_BYTES + 1).collect(),
        String::new(),
    );
    assert!(validate_launch_environment(&oversized).is_err());
}
