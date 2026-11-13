use crate::agent::AgentBinding;

pub(super) trait BindingRecoveryAuthority {
    fn authorizes(&self, binding: &AgentBinding) -> bool;
}
