use std::collections::BTreeMap;
use yolo_agent_contract::AgentKind;

/// Closed provider launch plans. The Hub resolves only these provider ids;
/// callers never supply argv or environment.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LaunchPolicies {
    commands: BTreeMap<AgentKind, String>,
}

impl LaunchPolicies {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, kind: AgentKind, command: impl Into<String>) {
        self.commands.insert(kind, command.into());
    }

    pub fn command(&self, provider_id: &str) -> Option<&str> {
        self.commands
            .get(&AgentKind::from_name(provider_id))
            .map(String::as_str)
    }

    /// Stable kind/executable pairs for runtime availability probing.
    pub fn entries(&self) -> impl Iterator<Item = (&AgentKind, &str)> {
        self.commands
            .iter()
            .map(|(kind, command)| (kind, command.as_str()))
    }
}
