use crate::hub::federation_support::{RunningNode, deadline};
use std::collections::BTreeSet;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_rpc_protocol::{Command, Reply};

pub(super) struct Identity {
    pub agent: AgentSnapshot,
    title: String,
    hub: String,
    address: String,
}

impl Identity {
    pub fn query(&self) -> String {
        self.agent.cursor().target().address.to_string()
    }

    pub fn rendered(&self, text: &str) -> bool {
        text.contains(&self.title) && text.contains(&self.hub) && text.contains(&self.address)
    }
}

pub(super) async fn expected(node: &RunningNode, names: &[&str]) -> Vec<Identity> {
    let agents = node.agents().await;
    assert_eq!(agents.len(), names.len());
    let mut client = node.client().await;
    let mut actual_names = BTreeSet::new();
    let mut identities = Vec::new();
    for agent in agents {
        let fields = agent.fields();
        let address = fields.terminal.address;
        let Reply::Terminal { terminal: session } = client
            .call(address.hub_id, Command::Get { address }, deadline())
            .await
            .unwrap()
        else {
            panic!("federated agent is missing its origin terminal");
        };
        assert_eq!(
            session.fields().agent,
            Some(agent.cursor().target())
        );
        assert_eq!(session.fields().runtime.unwrap().target, fields.terminal);
        actual_names.insert(session.fields().name.clone());
        let title = session
            .fields()
            .presentation
            .title
            .as_deref()
            .filter(|title| !title.is_empty())
            .unwrap_or(fields.provider.kind())
            .to_owned();
        identities.push(Identity {
            title,
            hub: address.hub_id.to_string(),
            address: fields.cursor.target().address.agent_id.to_string(),
            agent,
        });
    }
    assert_eq!(
        actual_names,
        names.iter().map(|name| (*name).to_owned()).collect()
    );
    identities
}
