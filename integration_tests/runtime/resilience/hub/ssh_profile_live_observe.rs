use super::{POLL, constants::*, deadline, process::RelayProcess};
use tokio::time::timeout_at;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Lifecycle, Reachability};
use yolo_hub_identity::{AgentTarget, HubId, RuntimeTarget};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_ssh_profile_protocol::{SshProfileChange, SshProfileConfiguration, SshProfileState};
pub async fn connect(
    node: &RelayProcess,
    hub: HubId,
    profile: &yolo_hub_ssh_profile_protocol::SshProfileRef,
) -> anyhow::Result<yolo_broker_hub_protocol::BrokerHubConnectionBinding> {
    let reply = node
        .client()
        .await
        .call(
            hub,
            Command::ConnectSshProfile {
                profile: profile.clone(),
                credentials: None,
            },
            deadline(),
        )
        .await?;
    let Reply::SshConnected { binding } = reply else {
        anyhow::bail!("Hub did not return an SSH broker binding")
    };
    binding.validate()?;
    Ok(binding)
}
pub async fn link(
    node: &RelayProcess,
    hub: HubId,
    configuration: yolo_hub_federation_settings::Configuration,
) {
    let Reply::Configuration { snapshot: before } = node
        .client()
        .await
        .call(hub, Command::GetConfiguration {}, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let change = yolo_hub_management_protocol::ConfigurationChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: before.state.revision,
        configuration,
    };
    let Reply::Configuration { snapshot } = node
        .client()
        .await
        .call(
            hub,
            Command::Configure {
                change: change.clone(),
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(snapshot.applied);
    assert_eq!(snapshot.state.last_change, change.request_id);
    assert_eq!(snapshot.state.configuration, change.configuration);
}

pub async fn profiles(node: &RelayProcess, hub: HubId) -> SshProfileState {
    let Reply::SshProfiles { state } = node
        .client()
        .await
        .call(hub, Command::GetSshProfiles {}, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(state.hub.hub(), hub);
    state.validate().unwrap();
    state
}
pub async fn configure(
    node: &RelayProcess,
    hub: HubId,
    configuration: SshProfileConfiguration,
) -> SshProfileState {
    let previous = profiles(node, hub).await;
    let change = SshProfileChange {
        request_id: uuid::Uuid::new_v4(),
        expected_revision: previous.revision,
        configuration,
    };
    let Reply::SshProfiles { state } = node
        .client()
        .await
        .call(
            hub,
            Command::ConfigureSshProfiles {
                change: change.clone(),
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(state.last_change, change.request_id);
    assert_eq!(state.configuration, change.configuration);
    state
}
pub async fn agent(
    node: &RelayProcess,
    entry: HubId,
    runtime: RuntimeTarget,
    reachable: bool,
) -> AgentSnapshot {
    timeout_at(deadline(), async {
        loop {
            let Reply::Agents { agents, next: None } = node
                .client()
                .await
                .call(entry, Command::ListAgents { after: None }, deadline())
                .await
                .unwrap()
            else {
                panic!()
            };
            let matches = agents
                .iter()
                .filter(|agent| agent.fields().terminal.address == runtime.address)
                .collect::<Vec<_>>();
            assert!(matches.len() <= SINGLE_AGENT);
            if let Some(agent) = matches.first() {
                assert_eq!(agent.fields().terminal, runtime);
                assert!(!matches!(
                    agent.fields().lifecycle,
                    Lifecycle::Exited { .. }
                ));
                let expected = if reachable {
                    (Freshness::Current, Reachability::Reachable)
                } else {
                    (Freshness::Stale, Reachability::Unreachable)
                };
                if (agent.fields().freshness, agent.fields().reachability) == expected {
                    return (*agent).clone();
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}
pub fn same_agent(snapshot: &AgentSnapshot, target: AgentTarget) {
    assert_eq!(snapshot.cursor().target(), target);
}
