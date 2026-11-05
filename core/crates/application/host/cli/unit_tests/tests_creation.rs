#[test]
fn create_requires_authorization_stdin() {
    assert!(
        Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
            cli::CREATE_COMMAND,
            cli::WORKING_DIRECTORY_ARGUMENT,
            "/tmp/project",
        ])
        .is_err()
    );
}

fn parsed_creation_authorization(command: &str) -> ctl_args::CreationAuthorization {
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        "/tmp/yolo",
        command,
        SESSION_ID,
        cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
    ])
    .unwrap();
    let Command::Ctl(CtlOptions { command, .. }) = parsed.command else {
        panic!("control command was not parsed");
    };
    match command {
        CtlCommand::CleanupTerminate { authorization, .. }
        | CtlCommand::CleanupArchive { authorization, .. }
        | CtlCommand::CleanupDiscard { authorization, .. }
        | CtlCommand::FinalizeCreation { authorization, .. } => authorization,
        _ => panic!("creation lifecycle command was not parsed"),
    }
}

#[test]
fn typed_creation_authority_rejects_tty_nil_unknown_and_oversized_input() {
    let capability = uuid::Uuid::new_v4();
    let encoded = yolo_capnp_codec::encode(&ctl_args::CleanupAuthority {
        creation_capability: capability,
    })
    .unwrap();
    let parse = || parsed_creation_authorization(cli::CLEANUP_ARCHIVE_COMMAND);
    assert_eq!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>(encoded.as_slice(), false)
            .unwrap()
            .validate()
            .unwrap()
            .creation_capability,
        capability
    );
    assert!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>(encoded.as_slice(), true)
            .is_err()
    );
    let nil = yolo_capnp_codec::encode(&ctl_args::CleanupAuthority {
        creation_capability: uuid::Uuid::nil(),
    })
    .unwrap();
    assert!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>(nil.as_slice(), false)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut unknown = serde_json::to_value(ctl_args::CleanupAuthority {
        creation_capability: capability,
    })
    .unwrap();
    unknown
        .as_object_mut()
        .unwrap()
        .insert(yolo_runtime::protocol::field::ERROR.to_owned(), true.into());
    let unknown = yolo_capnp_codec::encode(&unknown).unwrap();
    assert!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>(unknown.as_slice(), false)
            .is_err()
    );
    let oversized = vec![b'x'; ctl_args::MAX_CREATION_AUTHORITY_BYTES as usize + 1];
    assert!(
        parse()
            .read_checked::<ctl_args::CleanupAuthority>(oversized.as_slice(), false)
            .is_err()
    );
}

#[test]
fn create_and_finalize_decode_distinct_typed_authority_documents() {
    let token = uuid::Uuid::new_v4();
    let capability = uuid::Uuid::new_v4();
    let parsed = Cli::try_parse_from([
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        "/tmp/yolo",
        cli::CREATE_COMMAND,
        cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        cli::WORKING_DIRECTORY_ARGUMENT,
        "/tmp/project",
    ])
    .unwrap();
    let Command::Ctl(CtlOptions {
        command: CtlCommand::Create { authorization, .. },
        ..
    }) = parsed.command
    else {
        panic!("create command was not parsed");
    };
    let create = ctl_args::CreateAuthority {
        creation_token: token,
    };
    let encoded = yolo_capnp_codec::encode(&create).unwrap();
    assert_eq!(
        authorization
            .read_checked::<ctl_args::CreateAuthority>(encoded.as_slice(), false)
            .unwrap()
            .validate()
            .unwrap(),
        create
    );

    let finalize = ctl_args::FinalizeAuthority {
        creation_token: token,
        creation_capability: capability,
    };
    let encoded = yolo_capnp_codec::encode(&finalize).unwrap();
    assert_eq!(
        parsed_creation_authorization(cli::FINALIZE_CREATION_COMMAND)
            .read_checked::<ctl_args::FinalizeAuthority>(encoded.as_slice(), false)
            .unwrap()
            .validate()
            .unwrap(),
        finalize
    );
    let debug = format!("{finalize:?}");
    assert!(!debug.contains(&token.to_string()));
    assert!(!debug.contains(&capability.to_string()));
}

#[test]
fn creation_result_is_public_by_default_and_terminal_export_is_rejected() {
    let result = yolo_runtime::model::CreateTerminalResult {
        terminal_id: uuid::Uuid::new_v4(),
        creation_capability: uuid::Uuid::new_v4(),
    };
    let secret = result.creation_capability.to_string();
    let public = ctl::encode_creation_result(&result, false, true).unwrap();
    assert!(!String::from_utf8_lossy(&public).contains(&secret));
    assert_eq!(
        yolo_capnp_codec::decode::<serde_json::Value>(&public)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        1
    );
    let exported = ctl::encode_creation_result(&result, true, false).unwrap();
    assert!(String::from_utf8_lossy(&exported).contains(&secret));
    assert!(ctl::encode_creation_result(&result, true, true).is_err());
}

#[test]
fn typed_cli_authority_debug_never_contains_secret_uuids() {
    let capability = uuid::Uuid::new_v4();
    let encoded = yolo_capnp_codec::encode(&ctl_args::CleanupAuthority {
        creation_capability: capability,
    })
    .unwrap();
    let authority = parsed_creation_authorization(cli::CLEANUP_ARCHIVE_COMMAND)
        .read_checked::<ctl_args::CleanupAuthority>(encoded.as_slice(), false)
        .unwrap();
    assert!(!format!("{authority:?}").contains(&capability.to_string()));
}
