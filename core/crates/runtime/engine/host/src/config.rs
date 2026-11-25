use std::{path::PathBuf, sync::Arc};
use yolo_agent_providers::ProviderRuntimeRegistry;
#[derive(Debug, Clone)]
pub struct HostConfig {
    pub home: PathBuf,
    pub socket: Option<PathBuf>,
    pub runtime_executable: Option<PathBuf>,
    pub provider_runtime: Arc<ProviderRuntimeRegistry>,
}

impl HostConfig {
    pub fn new(home: impl Into<PathBuf>) -> Self {
        Self::with_provider_runtime(home, Arc::new(yolo_agent_providers::builtin_provider_runtime()))
    }

    pub fn with_provider_runtime(home: impl Into<PathBuf>, provider_runtime: Arc<ProviderRuntimeRegistry>) -> Self {
        Self { home: home.into(), socket: None, runtime_executable: None, provider_runtime }
    }
}
