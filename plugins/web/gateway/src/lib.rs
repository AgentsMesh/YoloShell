//! Loopback WebSocket transport for the authenticated public Hub API.
//! The backend authenticates to public-client.sock; same-UID management sockets
//! are never forwarded. Browser disconnect retires only its owned connection.
mod authentication;
mod backend;
mod bridge;
mod config;
mod constants;
mod handshake;
mod origin;
mod server;

pub use authentication::AccessToken;
pub use backend::ScopedHub;
pub use config::GatewayConfig;
pub use constants::*;
pub use origin::WebOrigin;
pub use server::{Gateway, accept_error_is_transient};
