//! Owned local tunnel; closing it revokes only this Unix connection.
mod custody;
mod stream;
pub use custody::BrokerTunnelClose;
pub use stream::BrokerHubTunnel;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
