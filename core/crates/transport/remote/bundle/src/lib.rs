//! Deploy complete bundles over one authenticated SSH transport, with one native commit.
mod command;
mod constants;
mod error;
mod executable;
mod installer;
mod probes;
mod recovery;
mod selection;

pub use error::UncertainPublication;
pub use installer::RemoteBundleInstaller;
