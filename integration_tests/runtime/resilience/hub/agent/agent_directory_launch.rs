use super::{
    super::{
        federation_support::{RunningNode, deadline},
        support::{acquire, input},
    },
    constants::*,
};
use crate::Harness;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    CreateShellFields, CreateShellRequest, LeaseMode, TERMINAL_PROTOCOL_SCHEMA_VERSION, TerminalSize,
};

pub fn prepare(harness: &Harness) -> (PathBuf, PathBuf, PathBuf) {
    let home = harness.home.path().canonicalize().unwrap();
    let bin = home.join(CLAUDE_BINARY_DIRECTORY);
    std::fs::create_dir(&bin).unwrap();
    let binary = bin.join(yolo_runtime::agent::claude::constants::EXECUTABLE);
    std::fs::copy(
        crate::bazel_runfile(crate::FAKE_CLAUDE_BINARY_RUNFILE).unwrap(),
        &binary,
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    let data = home.join(CLAUDE_DATA_DIRECTORY);
    let provider = crate::support::claude_install_spec();
    let preview = yolo_hook_installation::inspect(provider, &data).unwrap();
    yolo_hook_installation::install(provider, &data, &preview.revision).unwrap();
    let first = home.join(FIRST_DIRECTORY);
    let next = home.join(NEXT_DIRECTORY);
    for directory in [&first, &next] {
        std::fs::create_dir(directory).unwrap();
        std::fs::write(directory.join(PRIVATE_FILE), PRIVATE_CONTENT).unwrap();
    }
    (binary, first, next)
}

pub async fn launch(
    node: &RunningNode,
    harness: &Harness,
    binary: &Path,
    cwd: &Path,
) -> RuntimeTarget {
    let mut client = node.client().await;
    let token = uuid::Uuid::new_v4();
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: token,
        name: Some(SESSION_NAME.into()),
        cwd: harness
            .home
            .path()
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .into(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let Reply::Created { creation } = client
        .call(node.node.hub_id(), Command::Create { request }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let view = harness
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    harness.track_view(&view);
    client
        .call(
            creation.address.hub_id,
            Command::Finalize {
                creation_token: token,
                creation,
            },
            deadline(),
        )
        .await
        .unwrap();
    let Reply::Terminal { terminal: session } = client
        .call(
            creation.address.hub_id,
            Command::Get {
                address: creation.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let target = session.fields().runtime.unwrap().target;
    let Reply::Lease { grant } = client
        .call(
            target.address.hub_id,
            acquire(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let data = harness
        .home
        .path()
        .canonicalize()
        .unwrap()
        .join(CLAUDE_DATA_DIRECTORY);
    let command = format!(
        "cd '{}' && export {}='{}' && '{}'\r",
        quote(cwd),
        yolo_runtime::agent::claude::constants::DATA_HOME_ENV,
        quote(&data),
        quote(binary)
    );
    client
        .call(
            target.address.hub_id,
            input(grant, command.as_bytes()),
            deadline(),
        )
        .await
        .unwrap();
    client
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
    target
}

pub async fn change(node: &RunningNode, target: RuntimeTarget, cwd: &Path) {
    let mut client = node.client().await;
    let Reply::Lease { grant } = client
        .call(
            target.address.hub_id,
            acquire(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let command = format!(
        "{} {}\r",
        crate::fake_agent_constants::PACKAGED_TUI_CWD_COMMAND,
        serde_json::to_string(cwd.to_str().unwrap()).unwrap()
    );
    client
        .call(
            target.address.hub_id,
            input(grant, command.as_bytes()),
            deadline(),
        )
        .await
        .unwrap();
    client
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
}

fn quote(path: &Path) -> String {
    path.to_str().unwrap().replace('\'', SHELL_QUOTE_ESCAPE)
}
