use super::executable::HookExecutable;
use super::profile::HookCodec;
use std::path::PathBuf;
use std::sync::Arc;
use yolo_agent_authority_contract::AuthorityProcessIdentity;
use yolo_agent_contract::AgentBinding;
use yolo_agent_process::ProcessSource;
use yolo_agent_provider_runtime::ProviderRuntimeRegistry;
use yolo_paths::namespace_lease::NamespaceLease;
use yolo_process::ProcessIdentity;

/// Worker-owned context for the provider-neutral hook ingress. The Worker
/// supplies runtime identity, namespace, process observation and binding
/// publication; provider codecs supply wire decoding and evidence layout.
#[derive(Clone)]
pub struct HookIngressHost {
    pub terminal_id: uuid::Uuid,
    pub runtime_epoch: uuid::Uuid,
    pub worker: AuthorityProcessIdentity,
    pub boot_id: String,
    pub binding_path: PathBuf,
    pub namespace: Arc<NamespaceLease>,
    pub generation_gate: Arc<tokio::sync::Mutex<()>>,
    pub provider_runtime: Arc<ProviderRuntimeRegistry>,
    pub process_source: Arc<dyn ProcessSource>,
    pub shell: ProcessIdentity,
    pub codecs: Arc<Vec<Arc<dyn HookCodec>>>,
    pub publish: Arc<dyn Fn(&'static str, AgentBinding) + Send + Sync>,
    pub runtime_current: Arc<dyn Fn() -> bool + Send + Sync>,
    pub outbox: PathBuf,
}

pub(crate) struct Context {
    pub(crate) host: HookIngressHost,
    pub(crate) executable: Arc<HookExecutable>,
}

impl HookIngressHost {
    pub fn codec(&self, provider: &str) -> Option<Arc<dyn HookCodec>> {
        self.codecs
            .iter()
            .find(|codec| codec.profile().provider == provider)
            .cloned()
    }
}
