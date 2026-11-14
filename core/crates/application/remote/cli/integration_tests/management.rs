use super::*;
use yolo_remote_broker::{
    BROKER_PROTOCOL_VERSION, BrokerResult, BrokerStatus, DEFAULT_COMMAND_OUTPUT_BYTES,
    DEFAULT_COMMAND_TIMEOUT_MILLIS, WORKING_DIRECTORY_CAPABILITY,
};
use yolo_runtime::cli::constants;

#[tokio::test]
async fn command_and_file_cli_preserve_policy_and_report_transport_failure() {
    let (fixture, task) = running_fixture().await;
    let socket = BrokerPaths::new(&fixture.options.broker_state)
        .unwrap()
        .management_socket();
    let command = CommandOptions {
        socket: socket.clone(),
        program: fixture::TEST_PROGRAM.into(),
        cwd: None,
        timeout_millis: DEFAULT_COMMAND_TIMEOUT_MILLIS,
        output_limit: DEFAULT_COMMAND_OUTPUT_BYTES,
        args: vec![constants::STATUS_COMMAND.into()],
    };
    let error = run(Cli {
        command: Command::Exec(command),
    })
    .await
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains(fixture::EXPECTED_CONNECTION_ERROR)
    );
    let file = FileOptions {
        socket,
        path: fixture::TEST_READ_PATH.into(),
        offset: 0,
        length: 1,
    };
    assert!(
        run(Cli {
            command: Command::ReadFile(file)
        })
        .await
        .is_err()
    );
    output::print_result(BrokerResult::Status(BrokerStatus {
        protocol_version: BROKER_PROTOCOL_VERSION,
        adopted: true,
        instance_id: uuid::Uuid::nil(),
        host_id: fixture::TEST_HOST_ID.into(),
        connection_id: None,
        connection_generation: 0,
        config_revision: fixture::TEST_CONFIG_REVISION.into(),
        capabilities: vec![WORKING_DIRECTORY_CAPABILITY.into()],
    }))
    .unwrap();
    shutdown_running_fixture(&fixture, task).await;
}
