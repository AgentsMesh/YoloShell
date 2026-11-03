use super::*;
#[path = "protocol_ctl/support.rs"]
mod support;
use support::*;
use yolo_runtime::model::PROTOCOL_VERSION;

const CTL_SESSION_NAME: &str = "from-ctl";
const CTL_INPUT: &str = "ctl-data";
const CTL_COLUMNS: &str = "99";
const CTL_ROWS: &str = "31";
const CTL_SIGNAL: &str = "CONT";
const CTL_FINALIZATION_WAIT: Duration = Duration::from_millis(200);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn creation_authority_never_enters_live_process_arguments() {
    let harness = Harness::start().await;
    let terminal_id = Uuid::new_v4().to_string();
    let token = Uuid::new_v4().to_string();
    let capability = Uuid::new_v4().to_string();
    assert_ctl_arguments_hide_secrets(
        &harness,
        &[
            cli::CREATE_COMMAND,
            cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
            cli::WORKING_DIRECTORY_ARGUMENT,
            harness.home.path().to_str().unwrap(),
        ],
        std::slice::from_ref(&token),
    );
    for command in [cli::CLEANUP_ARCHIVE_COMMAND, cli::FINALIZE_CREATION_COMMAND] {
        assert_ctl_arguments_hide_secrets(
            &harness,
            &[
                command,
                &terminal_id,
                cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
            ],
            &[token.clone(), capability.clone()],
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn internal_ctl_commands_use_capnp_and_keep_authority_out_of_arguments() {
    let harness = Harness::start().await;
    let run = |arguments: &[&str]| run_ctl(&harness, arguments, None);
    let status = run(&[cli::STATUS_COMMAND]);
    assert_eq!(status[field::PROTOCOL_VERSION], PROTOCOL_VERSION);
    assert!(run(&[cli::LIST_COMMAND]).as_array().unwrap().is_empty());
    let creation_token = Uuid::new_v4().to_string();
    let creation_authority = yolo_capnp_codec::encode(&serde_json::json!({
        field::CREATION_TOKEN: creation_token,
    }))
    .unwrap();
    let create_arguments = [
        cli::CREATE_COMMAND,
        cli::CREATION_AUTHORIZATION_STDIN_ARGUMENT,
        cli::NAME_ARGUMENT,
        CTL_SESSION_NAME,
        cli::WORKING_DIRECTORY_ARGUMENT,
        harness.home.path().to_str().unwrap(),
    ];
    let public = run_ctl(&harness, &create_arguments, Some(&creation_authority));
    assert_eq!(public.as_object().unwrap().len(), 1);
    assert!(public.get(field::CREATION_CAPABILITY).is_none());
    let mut export_arguments = create_arguments.to_vec();
    export_arguments.push(cli::EXPORT_CREATION_AUTHORIZATION_ARGUMENT);
    let created = run_ctl(&harness, &export_arguments, Some(&creation_authority));
    assert!(created.get(field::CREATION_TOKEN).is_none());
    let id = created[field::TERMINAL_ID].as_str().unwrap();
    assert_eq!(
        run(&[cli::GET_COMMAND, id])[field::MANIFEST][field::NAME],
        CTL_SESSION_NAME
    );
    let grant = run(&[
        cli::ACQUIRE_COMMAND,
        id,
        cli::HOLDER_ARGUMENT,
        cli::CONTROL_COMMAND,
        cli::EXPORT_WRITER_AUTHORIZATION_ARGUMENT,
    ]);
    let authorization = yolo_capnp_codec::encode(&grant).unwrap();
    let authorized = |arguments: &[&str]| run_ctl(&harness, arguments, Some(&authorization));
    assert_eq!(
        authorized(&[
            cli::RESIZE_COMMAND,
            id,
            cli::WRITER_AUTHORIZATION_STDIN_ARGUMENT,
            cli::COLUMNS_ARGUMENT,
            CTL_COLUMNS,
            cli::ROWS_ARGUMENT,
            CTL_ROWS,
        ])[field::OK],
        true
    );
    assert_eq!(
        authorized(&[
            cli::INPUT_COMMAND,
            id,
            cli::WRITER_AUTHORIZATION_STDIN_ARGUMENT,
            cli::DATA_ARGUMENT,
            CTL_INPUT,
        ])[field::DELIVERY],
        delivery::DELIVERED
    );
    assert_eq!(run(&[cli::SNAPSHOT_COMMAND, id])[field::COLS], 99);
    assert_eq!(
        authorized(&[
            cli::SIGNAL_COMMAND,
            id,
            cli::WRITER_AUTHORIZATION_STDIN_ARGUMENT,
            CTL_SIGNAL,
        ])[field::OK],
        true
    );
    assert!(run(&[cli::ATTACH_COMMAND, id])[field::NEXT_SEQUENCE].is_number());
    assert!(run(&[cli::RECONCILE_COMMAND])[field::REATTACHED].is_number());
    assert_eq!(
        run(&[cli::TERMINATE_COMMAND, id, cli::FORCE_ARGUMENT])[field::OK],
        true
    );
    std::thread::sleep(CTL_FINALIZATION_WAIT);
    assert_eq!(
        run(&[cli::ARCHIVE_COMMAND, id])[field::MANIFEST][field::ARCHIVED],
        true
    );
}
