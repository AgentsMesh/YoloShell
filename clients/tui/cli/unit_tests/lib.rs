use crate::{arguments::Arguments, selection::Selection};
mod clients;
mod commands;
mod entry;
mod layouts;
mod lifecycle;
use clap::{Parser, error::ErrorKind};
use yolo_hub_client::{ClientError, HubEndpoint};
#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;

mod recipe;
mod recipe_edges;
mod remote_arguments;
mod remote_credentials;
mod remote_files;
#[path = "../test_support/remote_fixture.rs"]
mod remote_fixture;

#[test]
fn socket_and_local_endpoint_produce_the_same_options() {
    for selection in [
        [SOCKET_ARGUMENT, SOCKET],
        [ENDPOINT_ARGUMENT, LOCAL_ENDPOINT],
    ] {
        let arguments =
            Arguments::try_parse_from([PROGRAM, HUB_ARGUMENT, HUB, selection[0], selection[1]])
                .unwrap();
        let Selection::Explicit(options) = arguments.selection().unwrap() else {
            panic!()
        };
        assert_eq!(options.endpoint, HubEndpoint::local(SOCKET));
        assert_eq!(options.hub.to_string(), HUB);
    }
}

#[test]
fn empty_connection_selects_automatic_local_and_rejects_workspace() {
    let arguments = Arguments::try_parse_from([PROGRAM]).unwrap();
    assert!(matches!(
        arguments.selection().unwrap(),
        Selection::Local {
            profile: None,
            layout: None
        }
    ));
    assert_eq!(
        Arguments::try_parse_from([PROGRAM, WORKSPACE_ARGUMENT, WORKSPACE])
            .unwrap_err()
            .kind(),
        ErrorKind::UnknownArgument
    );
    let arguments = Arguments::try_parse_from([PROGRAM, LOCAL_PROFILE_ARGUMENT, PROFILE]).unwrap();
    assert!(matches!(arguments.selection().unwrap(),
        Selection::Local { profile: Some(path), layout: None } if path.to_str() == Some(PROFILE)));
}

#[test]
fn incomplete_or_conflicting_explicit_connection_never_falls_back() {
    for args in [
        vec![PROGRAM, HUB_ARGUMENT, HUB],
        vec![PROGRAM, SOCKET_ARGUMENT, SOCKET],
        vec![PROGRAM, ENDPOINT_ARGUMENT, LOCAL_ENDPOINT],
    ] {
        assert_eq!(
            Arguments::try_parse_from(args).unwrap_err().kind(),
            ErrorKind::MissingRequiredArgument
        );
    }
    for args in [
        vec![
            PROGRAM,
            HUB_ARGUMENT,
            HUB,
            SOCKET_ARGUMENT,
            SOCKET,
            ENDPOINT_ARGUMENT,
            LOCAL_ENDPOINT,
        ],
        vec![
            PROGRAM,
            HUB_ARGUMENT,
            HUB,
            SOCKET_ARGUMENT,
            SOCKET,
            LOCAL_PROFILE_ARGUMENT,
            PROFILE,
        ],
    ] {
        assert_eq!(
            Arguments::try_parse_from(args).unwrap_err().kind(),
            ErrorKind::ArgumentConflict
        );
    }
}

#[test]
fn unsupported_ssh_and_invalid_identifiers_fail_before_resolution() {
    let error =
        Arguments::try_parse_from([PROGRAM, HUB_ARGUMENT, HUB, ENDPOINT_ARGUMENT, SSH_ENDPOINT])
            .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::ValueValidation);
    assert!(
        error
            .to_string()
            .contains(&ClientError::UnsupportedEndpoint.to_string())
    );
    assert!(
        Arguments::try_parse_from([PROGRAM, HUB_ARGUMENT, INVALID_HUB, SOCKET_ARGUMENT, SOCKET],)
            .is_err()
    );
}

#[test]
fn unknown_schemes_and_invalid_paths_never_become_local_defaults() {
    assert!(
        Arguments::try_parse_from([
            PROGRAM,
            HUB_ARGUMENT,
            HUB,
            ENDPOINT_ARGUMENT,
            UNKNOWN_ENDPOINT
        ],)
        .is_err()
    );
    assert!(Arguments::try_parse_from([PROGRAM, HUB_ARGUMENT, HUB, SOCKET_ARGUMENT, ""],).is_err());
    let args =
        Arguments::try_parse_from([PROGRAM, HUB_ARGUMENT, HUB, SOCKET_ARGUMENT, CONTROL_PATH])
            .unwrap();
    assert!(args.selection().is_err());
    for argument in [LOCAL_PROFILE_ARGUMENT] {
        let args = Arguments::try_parse_from([PROGRAM, argument, RELATIVE_PATH]).unwrap();
        assert!(args.selection().is_err());
    }
}
