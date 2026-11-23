//! Trusted host for the bundled TUI client descriptor.
//!
//! This host validates built-in declarations and negotiates capabilities. It
//! does not load libraries or execute manifest-supplied paths.

mod error;
mod registry;

pub use error::ClientHostError;
pub use registry::{BuiltinClientHost, ClientSessionDeclaration};

pub trait BuiltinClientEntrypoint {
    fn id(&self) -> &'static str;
    fn run(&self, arguments: Vec<std::ffi::OsString>) -> anyhow::Result<()>;
}

pub fn launch<P: BuiltinClientEntrypoint>(
    host: &BuiltinClientHost,
    client: P,
    arguments: Vec<std::ffi::OsString>,
) -> anyhow::Result<()> {
    let descriptor = host
        .get(client.id())
        .ok_or_else(|| anyhow::anyhow!("built-in client is not registered"))?;
    anyhow::ensure!(
        descriptor
            .platform()
            .runs_on(yolo_plugin_api::Platform::current()),
        "built-in client platform mismatch"
    );
    client.run(arguments)
}
