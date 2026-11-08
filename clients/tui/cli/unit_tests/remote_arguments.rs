use super::constants::{
    ENDPOINT_ARGUMENT, HUB, HUB_ARGUMENT, LOCAL_ENDPOINT, LOCAL_PROFILE_ARGUMENT, PROFILE, PROGRAM,
    SOCKET, SOCKET_ARGUMENT,
};
use crate::arguments::Arguments;
use clap::{Parser, error::ErrorKind};

#[path = "../test_support/remote_arguments_constants.rs"]
mod constants;
use constants::*;

fn complete_arguments() -> Vec<&'static str> {
    vec![
        PROGRAM,
        REMOTE_PROFILE_ARGUMENT,
        REMOTE_PROFILE,
        ACCOUNT_KEY_ARGUMENT,
        ACCOUNT_KEY,
        CLIENT_KEY_ARGUMENT,
        CLIENT_KEY,
    ]
}

#[test]
fn complete_remote_credentials_parse_without_opening_profile_or_key_files() {
    assert!(Arguments::try_parse_from(complete_arguments()).is_ok());
    assert!(
        Arguments::try_parse_from([
            PROGRAM,
            CLIENT_KEY_ARGUMENT,
            CLIENT_KEY,
            ACCOUNT_KEY_ARGUMENT,
            ACCOUNT_KEY,
            REMOTE_PROFILE_ARGUMENT,
            REMOTE_PROFILE,
        ])
        .is_ok()
    );
}

#[test]
fn remote_profile_requires_both_distinct_credential_arguments() {
    for supplied in [
        Vec::new(),
        vec![ACCOUNT_KEY_ARGUMENT, ACCOUNT_KEY],
        vec![CLIENT_KEY_ARGUMENT, CLIENT_KEY],
    ] {
        let mut arguments = vec![PROGRAM, REMOTE_PROFILE_ARGUMENT, REMOTE_PROFILE];
        arguments.extend(supplied);
        assert_eq!(
            Arguments::try_parse_from(arguments).unwrap_err().kind(),
            ErrorKind::MissingRequiredArgument
        );
    }
}

#[test]
fn credential_arguments_without_a_remote_profile_never_select_local_startup() {
    for supplied in [
        vec![ACCOUNT_KEY_ARGUMENT, ACCOUNT_KEY],
        vec![CLIENT_KEY_ARGUMENT, CLIENT_KEY],
        vec![
            ACCOUNT_KEY_ARGUMENT,
            ACCOUNT_KEY,
            CLIENT_KEY_ARGUMENT,
            CLIENT_KEY,
        ],
    ] {
        let mut arguments = vec![PROGRAM];
        arguments.extend(supplied);
        assert_eq!(
            Arguments::try_parse_from(arguments).unwrap_err().kind(),
            ErrorKind::MissingRequiredArgument
        );
    }
}

#[test]
fn remote_entry_conflicts_with_each_local_selector_and_complete_explicit_connections() {
    for local in [
        vec![HUB_ARGUMENT, HUB],
        vec![SOCKET_ARGUMENT, SOCKET],
        vec![ENDPOINT_ARGUMENT, LOCAL_ENDPOINT],
        vec![LOCAL_PROFILE_ARGUMENT, PROFILE],
        vec![HUB_ARGUMENT, HUB, SOCKET_ARGUMENT, SOCKET],
        vec![HUB_ARGUMENT, HUB, ENDPOINT_ARGUMENT, LOCAL_ENDPOINT],
    ] {
        let mut arguments = complete_arguments();
        arguments.extend(local);
        assert_eq!(
            Arguments::try_parse_from(arguments).unwrap_err().kind(),
            ErrorKind::ArgumentConflict
        );
    }
}
