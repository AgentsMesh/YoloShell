use yolo_builtin_client_contract::ClientDescriptorError;
use yolo_plugin_api::Platform;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ClientHostError {
    #[error("unknown built-in client")]
    UnknownClient,
    #[error("built-in client is already registered")]
    DuplicateClient,
    #[error("invalid built-in client descriptor: {0}")]
    Descriptor(#[from] ClientDescriptorError),
    #[error("built-in client startup requires unsupported hub methods")]
    StartupUnsupported,
    #[error("built-in client cannot run on the requested platform: {0:?}")]
    UnsupportedPlatform(Platform),
}
