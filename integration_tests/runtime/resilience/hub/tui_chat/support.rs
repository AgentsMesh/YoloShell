use super::constants::*;
use crate::{
    Harness,
    hub::support::{RunningHub, acquire, deadline, input},
};
use std::{os::unix::fs::PermissionsExt, path::Path};
use yolo_hub_agent_protocol::{AgentSnapshot, Capability};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

pub async fn launch(
    host: &Harness,
    hub: &RunningHub,
    marker: &Path,
    provider: &yolo_hook_installation::HookInstallSpec,
) -> RuntimeTarget {
    let root = host.home.path().canonicalize().unwrap();
    let bin = root.join(BINARY_DIRECTORY);
    std::fs::create_dir(&bin).unwrap();
    let (executable, runfile, data_environment) = super::provider::launch_values(provider);
    let binary = bin.join(executable);
    std::fs::copy(crate::bazel_runfile(runfile).unwrap(), &binary).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(EXECUTABLE_MODE)).unwrap();
    let data = root.join(DATA_DIRECTORY);
    let preview = yolo_hook_installation::inspect(provider, &data).unwrap();
    yolo_hook_installation::install(provider, &data, &preview.revision).unwrap();
    let mut client = hub.client().await;
    let token = uuid::Uuid::new_v4();
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: token,
        name: Some(NAME.into()),
        cwd: root.to_str().unwrap().into(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let Reply::Created { creation } = client
        .call(hub.hub, Command::Create { request }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let view = host
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    host.track_view(&view);
    client
        .call(
            hub.hub,
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
            hub.hub,
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
        .call(hub.hub, acquire(target, LeaseMode::Acquire {}), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let command = format!(
        "export {}='{}' {}=1 {}=1 {}='{}'; '{}'\r",
        data_environment,
        quote(&data),
        crate::fake_agent_constants::PACKAGED_E2E_ENV,
        ENABLE_ENV,
        MARKER_ENV,
        quote(marker),
        quote(&binary)
    );
    client
        .call(hub.hub, input(grant, command.as_bytes()), deadline())
        .await
        .unwrap();
    client
        .call(hub.hub, Command::Release { grant }, deadline())
        .await
        .unwrap();
    current(hub, target).await;
    target
}

pub async fn current(hub: &RunningHub, target: RuntimeTarget) -> AgentSnapshot {
    observed(hub, target, |agent| {
        agent.fields().capabilities.submit_prompt == Capability::Available {}
    })
    .await
}

pub async fn observed(
    hub: &RunningHub,
    target: RuntimeTarget,
    accepts: impl Fn(&AgentSnapshot) -> bool,
) -> AgentSnapshot {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let Reply::Agents { agents, .. } = hub
                .client()
                .await
                .call(hub.hub, Command::ListAgents { after: None }, deadline())
                .await
                .unwrap()
            else {
                panic!()
            };
            if let Some(agent) = agents
                .into_iter()
                .find(|agent| agent.fields().terminal == target && accepts(agent))
            {
                return agent;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}
fn quote(path: &Path) -> String {
    path.to_str().unwrap().replace('\'', QUOTE_ESCAPE)
}
