use crate::{ProjectionPolicy, ProviderDescriptor};
use std::collections::BTreeMap;
use yolo_agent_contract::AgentKind;

/// Projection policy data selected by the compiled-in provider catalog. This is
/// policy input to the Hub projection mechanism, never a provider branch.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProjectionPolicies {
    policies: BTreeMap<AgentKind, ProjectionPolicy>,
    display_names: BTreeMap<AgentKind, String>,
}

impl ProjectionPolicies {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn from_descriptors<'a>(
        descriptors: impl IntoIterator<Item = &'a ProviderDescriptor>,
    ) -> Self {
        let mut policies = BTreeMap::new();
        let mut display_names = BTreeMap::new();
        for descriptor in descriptors {
            policies.insert(descriptor.kind().clone(), descriptor.projection_policy());
            display_names.insert(descriptor.kind().clone(), descriptor.display_name().to_owned());
        }
        Self {
            policies,
            display_names,
        }
    }

    pub fn insert(&mut self, kind: AgentKind, policy: ProjectionPolicy) {
        self.policies.insert(kind, policy);
    }

    pub fn insert_display_name(&mut self, kind: AgentKind, display_name: String) {
        self.display_names.insert(kind, display_name);
    }

    /// Provider-owned display name, absent for kinds outside the catalog.
    pub fn display_name(&self, kind: &AgentKind) -> Option<&str> {
        self.display_names.get(kind).map(String::as_str)
    }

    pub fn policy(&self, kind: &AgentKind) -> ProjectionPolicy {
        self.policies
            .get(kind)
            .copied()
            .unwrap_or(ProjectionPolicy::PROCESS_OBSERVER)
    }

    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }
}
