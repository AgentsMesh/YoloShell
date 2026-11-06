//! Provider runtime composition. Concrete providers are registered by the
//! process composition root; consumers receive this registry by injection.
mod error;
mod hook_client;
mod registry;
mod runtime;
mod turn_end;

pub use error::ProviderRuntimeError;
pub use hook_client::HookClient;
pub use registry::ProviderRuntimeRegistry;
pub use runtime::ProviderRuntime;
pub use turn_end::{TurnEndKind, TurnEndProbe, TurnEndRecord};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
