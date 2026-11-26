use super::*;
use crate::unit_support::*;
use std::collections::BTreeMap;

const ASSIGNMENT_SEPARATOR: char = '=';

#[test]
fn string_storage_retains_arguments_and_a_single_null_terminator() {
    let vector = strings([FIRST_ARGUMENT.to_owned(), SECOND_ARGUMENT.to_owned()]).unwrap();
    assert_eq!(decode_vector(&vector), [FIRST_ARGUMENT, SECOND_ARGUMENT]);
    let empty = strings(Vec::new()).unwrap();
    assert!(decode_vector(&empty).is_empty());
    assert_eq!(empty.pointers, vec![std::ptr::null_mut()]);
}

#[test]
fn nul_arguments_and_executable_names_are_rejected() {
    assert!(strings([NUL_ARGUMENT.to_owned()]).is_err());
    let fixture = Fixture::new();
    let mut command = fixture.command();
    command.arguments = vec![NUL_ARGUMENT.into()];
    assert!(argument_vector(&command).is_err());
    command.arguments.clear();
    command.executable = NUL_ARGUMENT.into();
    assert!(argument_vector(&command).is_err());
}

#[test]
fn argv_is_ordered_and_the_environment_contains_only_fixed_bindings() {
    let fixture = Fixture::new();
    let mut command = fixture.command();
    command.arguments = vec![FIRST_ARGUMENT.into(), SECOND_ARGUMENT.into()];
    assert_eq!(
        decode_vector(&argument_vector(&command).unwrap()),
        [EXECUTABLE, FIRST_ARGUMENT, SECOND_ARGUMENT]
    );
    let capability = ProcessDiscoveryCapability::generate().unwrap();
    let values = decode_vector(&environment_vector(&command, &capability).unwrap());
    let environment: BTreeMap<_, _> = values
        .iter()
        .map(|value| value.split_once(ASSIGNMENT_SEPARATOR).unwrap())
        .collect();
    assert_eq!(values.len(), environment.len());
    let expected = BTreeMap::from([
        (PATH_ENVIRONMENT, FIXED_SYSTEM_PATH.to_owned()),
        (
            HOME_ENVIRONMENT,
            command.home.display_path().display().to_string(),
        ),
        (
            TEMP_ENVIRONMENT,
            command
                .temporary_directory
                .display_path()
                .display()
                .to_string(),
        ),
        (LANG_ENVIRONMENT, DETERMINISTIC_LOCALE.to_owned()),
        (LC_ALL_ENVIRONMENT, DETERMINISTIC_LOCALE.to_owned()),
        (
            GIT_CONFIG_NOSYSTEM_ENVIRONMENT,
            ENABLED_ENVIRONMENT_VALUE.to_owned(),
        ),
        (
            GIT_ATTR_NOSYSTEM_ENVIRONMENT,
            ENABLED_ENVIRONMENT_VALUE.to_owned(),
        ),
        (GIT_CONFIG_GLOBAL_ENVIRONMENT, NULL_DEVICE.to_owned()),
        (GIT_CONFIG_SYSTEM_ENVIRONMENT, NULL_DEVICE.to_owned()),
        (
            GIT_NO_REPLACE_OBJECTS_ENVIRONMENT,
            ENABLED_ENVIRONMENT_VALUE.to_owned(),
        ),
        (
            GIT_OPTIONAL_LOCKS_ENVIRONMENT,
            DISABLED_ENVIRONMENT_VALUE.to_owned(),
        ),
        (
            PROCESS_DISCOVERY_CAPABILITY_ENV,
            capability.value().to_owned(),
        ),
    ]);
    assert_eq!(
        environment
            .into_iter()
            .map(|(key, value)| (key, value.to_owned()))
            .collect::<BTreeMap<_, _>>(),
        expected
    );
}
