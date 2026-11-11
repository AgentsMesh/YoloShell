use yolo_runtime::model::{WriterCapability, WriterLease, WriterLeaseGrant};
use yolo_runtime::worker::RuntimeRef;

const TEST_HOLDER: &str = "desktop";
const TEST_GENERATION: u64 = 7;
const TEST_EXPIRY_MS: i64 = 8;
const TEST_TTL_MS: u64 = 9;
const TEST_RENEW_AFTER_MS: u64 = 3;
const TEST_RUNTIME_EPOCH_VALUE: u128 = 1;

fn test_target() -> RuntimeRef {
    RuntimeRef::new(
        uuid::Uuid::parse_str(SESSION_ID).unwrap(),
        uuid::Uuid::from_u128(TEST_RUNTIME_EPOCH_VALUE),
    )
}

fn test_grant() -> WriterLeaseGrant {
    WriterLeaseGrant {
        lease: WriterLease {
            holder_id: TEST_HOLDER.into(),
            controller_instance_id: None,
            generation: TEST_GENERATION,
            expires_at_ms: TEST_EXPIRY_MS,
            ttl_ms: TEST_TTL_MS,
            renew_after_ms: TEST_RENEW_AFTER_MS,
        },
        writer_capability: WriterCapability::generate(),
    }
}

fn test_authorization() -> ctl_args::RuntimeWriterAuthorization {
    ctl_args::RuntimeWriterAuthorization::new(test_target(), test_grant())
}

#[test]
fn terminal_mutations_require_authorization_on_stdin() {
    let authorization_argument = ctl_args::writer_authorization_cli_argument();
    for tail in [
        vec![cli::INPUT_COMMAND, SESSION_ID, cli::DATA_ARGUMENT, "x"],
        vec![
            cli::RESIZE_COMMAND,
            SESSION_ID,
            cli::COLUMNS_ARGUMENT,
            "80",
            cli::ROWS_ARGUMENT,
            "24",
        ],
        vec![cli::SIGNAL_COMMAND, SESSION_ID, "INT"],
    ] {
        let mut arguments = vec![
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
        ];
        arguments.extend(tail);
        assert!(Cli::try_parse_from(arguments.clone()).is_err());
        arguments.push(authorization_argument.as_str());
        assert!(Cli::try_parse_from(arguments).is_ok());
    }
}

#[test]
fn grant_export_and_takeover_require_explicit_safe_arguments() {
    let prefix = [
        yolo_launch_contract::EXECUTABLE_NAME,
        cli::CONTROL_COMMAND,
        cli::HOME_ARGUMENT,
        "/tmp/yolo",
    ];
    assert!(
        Cli::try_parse_from(prefix.into_iter().chain([
            cli::ACQUIRE_COMMAND,
            SESSION_ID,
            cli::HOLDER_ARGUMENT,
            TEST_HOLDER
        ]))
        .is_ok()
    );
    assert!(
        Cli::try_parse_from(prefix.into_iter().chain([
            cli::TAKEOVER_COMMAND,
            SESSION_ID,
            cli::HOLDER_ARGUMENT,
            TEST_HOLDER,
        ]))
        .is_err()
    );
    assert!(
        Cli::try_parse_from(prefix.into_iter().chain([
            cli::TAKEOVER_COMMAND,
            SESSION_ID,
            cli::HOLDER_ARGUMENT,
            TEST_HOLDER,
            cli::OBSERVED_GENERATION_ARGUMENT,
            "7",
            cli::EXPORT_WRITER_AUTHORIZATION_ARGUMENT,
        ]))
        .is_ok()
    );
}

#[test]
fn grant_output_is_public_by_default_and_refuses_terminal_export() {
    let grant = test_grant();
    let target = test_target();
    let secret = serde_json::to_value(&grant).unwrap()
        [yolo_runtime::protocol::field::WRITER_CAPABILITY]
        .as_str()
        .unwrap()
        .to_owned();
    let public = ctl::encode_writer_grant(target, &grant, false, true).unwrap();
    assert!(!String::from_utf8_lossy(&public).contains(&secret));
    assert!(String::from_utf8_lossy(&public).contains(&target.runtime_epoch.to_string()));
    let exported = ctl::encode_writer_grant(target, &grant, true, false).unwrap();
    assert!(String::from_utf8_lossy(&exported).contains(&secret));
    assert!(ctl::encode_writer_grant(target, &grant, true, true).is_err());
}

#[test]
fn stdin_authorization_decodes_a_typed_grant_and_rejects_invalid_json() {
    let parse = || {
        let parsed = Cli::try_parse_from([
            yolo_launch_contract::EXECUTABLE_NAME,
            cli::CONTROL_COMMAND,
            cli::HOME_ARGUMENT,
            "/tmp/yolo",
            cli::INPUT_COMMAND,
            SESSION_ID,
            cli::DATA_ARGUMENT,
            "x",
            cli::WRITER_AUTHORIZATION_STDIN_ARGUMENT,
        ])
        .unwrap();
        let Command::Ctl(CtlOptions {
            command: CtlCommand::Input { authorization, .. },
            ..
        }) = parsed.command
        else {
            panic!("input command was not parsed");
        };
        authorization
    };
    let authorization = test_authorization();
    let encoded = yolo_capnp_codec::encode(&authorization).unwrap();
    assert_eq!(
        parse().read_checked(encoded.as_slice(), false).unwrap(),
        authorization
    );
    assert!(
        parse()
            .read_checked(yolo_capnp_codec::encode(&test_grant()).unwrap().as_slice(), false)
            .is_err()
    );
    assert!(parse().read_checked(b"not-json".as_slice(), false).is_err());
    assert!(parse().read_checked(encoded.as_slice(), true).is_err());
    let mut nil = test_authorization();
    nil.target.runtime_epoch = uuid::Uuid::nil();
    assert!(
        parse()
            .read_checked(yolo_capnp_codec::encode(&nil).unwrap().as_slice(), false)
            .is_err()
    );
    assert!(
        parse()
            .read_checked(
                vec![b'x'; ctl_args::MAX_WRITER_AUTHORIZATION_BYTES as usize + 1].as_slice(),
                false,
            )
            .is_err()
    );
    assert!(
        ctl::validate_mutation_authorization(uuid::Uuid::new_v4(), test_authorization()).is_err()
    );
}
