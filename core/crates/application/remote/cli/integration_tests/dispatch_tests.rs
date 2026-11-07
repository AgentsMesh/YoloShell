use super::super::{Cli, Command, CommandOptions, FileOptions, run};
use super::fixture;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use tokio::net::UnixListener;
use yolo_remote_broker::{
    BROKER_PROTOCOL_VERSION, BrokerRequest, BrokerRequestEnvelope, BrokerResponse, BrokerResult,
    BrokerStatus, CONNECTION_BOOTSTRAP_CAPABILITY,
};
use yolo_runtime::protocol::{read_frame, write_frame};

struct ReplyServer {
    _root: tempfile::TempDir,
    path: PathBuf,
    task: tokio::task::JoinHandle<BrokerRequest>,
}

impl ReplyServer {
    async fn start(result: BrokerResult) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(fixture::MANAGEMENT_SOCKET_FILE);
        let listener = UnixListener::bind(&path).unwrap();
        fs::set_permissions(
            &path,
            fs::Permissions::from_mode(fixture::PRIVATE_FILE_MODE),
        )
        .unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let request: BrokerRequestEnvelope = read_frame(&mut stream).await.unwrap();
            let response = BrokerResponse {
                protocol_version: BROKER_PROTOCOL_VERSION,
                request_id: request.request_id,
                result: Some(result),
                error_code: None,
                error_message: None,
            };
            write_frame(&mut stream, &response).await.unwrap();
            request.request
        });
        Self {
            _root: root,
            path,
            task,
        }
    }

    async fn finish(self) -> BrokerRequest {
        self.task.await.unwrap()
    }
}

fn decoded_result(value: serde_json::Value) -> BrokerResult {
    serde_json::from_value(value).unwrap()
}

#[tokio::test]
async fn successful_management_dispatch_preserves_each_request_shape() {
    let connection_id = uuid::Uuid::from_u128(fixture::TEST_CONNECTION_ID);
    let bootstrap = ReplyServer::start(BrokerResult::Bootstrap(BrokerStatus {
        protocol_version: BROKER_PROTOCOL_VERSION,
        adopted: true,
        instance_id: uuid::Uuid::new_v4(),
        host_id: fixture::TEST_HOST_ID.into(),
        connection_id: Some(connection_id),
        connection_generation: fixture::TEST_CONNECTION_GENERATION,
        config_revision: fixture::TEST_CONFIG_REVISION.into(),
        capabilities: vec![CONNECTION_BOOTSTRAP_CAPABILITY.into()],
    }))
    .await;
    run(Cli {
        command: Command::Bootstrap {
            socket: bootstrap.path.clone(),
        },
    })
    .await
    .unwrap();
    assert_eq!(bootstrap.finish().await, BrokerRequest::Bootstrap);

    let working = ReplyServer::start(BrokerResult::WorkingDirectory {
        connection_id,
        path: fixture::TEST_COMMAND_CWD.into(),
    })
    .await;
    run(Cli {
        command: Command::WorkingDirectory {
            socket: working.path.clone(),
        },
    })
    .await
    .unwrap();
    assert_eq!(working.finish().await, BrokerRequest::WorkingDirectory);

    let command_result = decoded_result(serde_json::json!({
        "command": {
            (fixture::CONNECTION_ID_FIELD): connection_id,
            "output": {
                "stdout": fixture::TEST_COMMAND_STDOUT,
                "stderr": [],
                "exit_status": fixture::TEST_COMMAND_EXIT_STATUS,
                "timed_out": false,
                "truncated": false
            }
        }
    }));
    let command = ReplyServer::start(command_result).await;
    run(Cli {
        command: Command::Exec(CommandOptions {
            socket: command.path.clone(),
            program: fixture::TEST_PROGRAM.into(),
            cwd: Some(fixture::TEST_COMMAND_CWD.into()),
            timeout_millis: fixture::TEST_COMMAND_TIMEOUT_MILLIS,
            output_limit: fixture::TEST_COMMAND_OUTPUT_BYTES,
            args: vec![fixture::TEST_COMMAND_ARGUMENT.into()],
        }),
    })
    .await
    .unwrap();
    let BrokerRequest::Command(request) = command.finish().await else {
        panic!("command dispatch changed request kind")
    };
    assert_eq!(request.program, fixture::TEST_PROGRAM);
    assert_eq!(request.args, [fixture::TEST_COMMAND_ARGUMENT]);
    assert_eq!(request.cwd, Some(fixture::TEST_COMMAND_CWD.into()));
    assert_eq!(request.timeout_millis, fixture::TEST_COMMAND_TIMEOUT_MILLIS);
    assert_eq!(request.output_limit, fixture::TEST_COMMAND_OUTPUT_BYTES);

    let file_result = decoded_result(serde_json::json!({
        (fixture::FILE_READ_RESULT_FIELD): {
            (fixture::CONNECTION_ID_FIELD): connection_id,
            "result": {
                (fixture::RESOLVED_PATH_FIELD): fixture::TEST_READ_PATH,
                "data": fixture::TEST_FILE_BYTES,
                "eof": true
            }
        }
    }));
    let file = ReplyServer::start(file_result).await;
    run(Cli {
        command: Command::ReadFile(FileOptions {
            socket: file.path.clone(),
            path: fixture::TEST_READ_PATH.into(),
            offset: fixture::TEST_FILE_OFFSET,
            length: fixture::TEST_FILE_LENGTH,
        }),
    })
    .await
    .unwrap();
    let BrokerRequest::FileRead(request) = file.finish().await else {
        panic!("file dispatch changed request kind")
    };
    assert_eq!(request.path, PathBuf::from(fixture::TEST_READ_PATH));
    assert_eq!(request.offset, fixture::TEST_FILE_OFFSET);
    assert_eq!(request.length, fixture::TEST_FILE_LENGTH);
}
