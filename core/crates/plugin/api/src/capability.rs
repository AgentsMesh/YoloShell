use serde::{Deserialize, Serialize};
use yolo_hub_rpc_protocol::Method;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientCapability {
    Layouts,
    TerminalControl,
    TerminalLifecycle,
    AgentObservation,
    AgentControl,
    Conversations,
    HubManagement,
    FileAccess,
    RemoteSetup,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientFeatureRequirement {
    pub capability: ClientCapability,
    pub methods: Vec<Method>,
}

impl ClientFeatureRequirement {
    pub fn new(capability: ClientCapability, methods: impl Into<Vec<Method>>) -> Self {
        Self {
            capability,
            methods: methods.into(),
        }
    }

    pub fn all() -> Vec<Self> {
        [
            ClientCapability::Layouts,
            ClientCapability::TerminalControl,
            ClientCapability::TerminalLifecycle,
            ClientCapability::AgentObservation,
            ClientCapability::AgentControl,
            ClientCapability::Conversations,
            ClientCapability::HubManagement,
            ClientCapability::FileAccess,
            ClientCapability::RemoteSetup,
        ]
        .into_iter()
        .map(Self::for_capability)
        .collect()
    }

    pub fn for_capability(capability: ClientCapability) -> Self {
        let methods = match capability {
            ClientCapability::Layouts => {
                vec![Method::ListLayouts, Method::GetLayout, Method::UpdateLayout]
            }
            ClientCapability::TerminalControl => vec![
                Method::ObserveLease,
                Method::Lease,
                Method::Renew,
                Method::Release,
                Method::Input,
                Method::Resize,
            ],
            ClientCapability::TerminalLifecycle => vec![
                Method::Create,
                Method::QueryCreation,
                Method::Finalize,
                Method::Terminal,
            ],
            ClientCapability::AgentObservation => {
                vec![
                    Method::ListAgents,
                    Method::GetAgent,
                    Method::GetCurrentAgent,
                ]
            }
            ClientCapability::AgentControl => vec![Method::AgentControl, Method::ListAgents],
            ClientCapability::Conversations => vec![
                Method::ListAgents,
                Method::ResolveConversation,
                Method::ReadConversation,
                Method::WatchConversation,
            ],
            ClientCapability::HubManagement => {
                vec![
                    Method::ListHubs,
                    Method::GetConfiguration,
                    Method::Configure,
                ]
            }
            ClientCapability::FileAccess => vec![Method::ListFileRoots, Method::ReadFile],
            ClientCapability::RemoteSetup => vec![
                Method::GetSshProfiles,
                Method::ConfigureSshProfiles,
                Method::ConnectSshProfile,
                Method::RemoteSetup,
                Method::InspectRemoteSetup,
            ],
        };
        Self::new(capability, methods)
    }
}
