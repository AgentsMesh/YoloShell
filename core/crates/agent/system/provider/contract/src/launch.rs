use std::collections::BTreeMap;
use yolo_agent_contract::AgentKind;

/// Closed provider launch plans. The Hub resolves only these provider ids;
/// callers never supply argv or environment.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LaunchPolicies {
    commands: BTreeMap<AgentKind, (String, Option<String>)>,
}

impl LaunchPolicies {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, kind: AgentKind, command: impl Into<String>) {
        self.commands.insert(kind, (command.into(), None));
    }

    pub fn insert_versioned(&mut self, kind: AgentKind, command: &str, previous: Option<&str>) {
        self.commands.insert(kind, (command.into(), previous.map(str::to_owned)));
    }

    pub fn command(&self, provider_id: &str) -> Option<&str> {
        self.commands
            .get(&AgentKind::from_name(provider_id))
            .map(|(command, _)| command.as_str())
    }

    pub fn previous_command(&self, provider_id: &str) -> Option<&str> {
        self.commands.get(&AgentKind::from_name(provider_id))?.1.as_deref()
    }

    /// Current launch commands ordered by provider kind.
    pub fn entries(&self) -> impl Iterator<Item = (&AgentKind, &str)> {
        self.commands
            .iter()
            .map(|(kind, (command, _))| (kind, command.as_str()))
    }
}
