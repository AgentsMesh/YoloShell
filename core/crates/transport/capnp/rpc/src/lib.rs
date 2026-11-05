//! Bounded Send-facing adapters around one shared Cap'n Proto LocalSet executor.
mod api;
mod client;
mod confirmation_drain;
mod connection;
mod constants;
mod domain;
mod engine;
mod event_client;
mod receipt;
mod receipt_client;
mod server;
mod server_events;
mod server_shutdown;

pub use api::{Confirmation, Duplex, Error, EventSource, Future, Method, Protocol, Reply, Service};
pub use connection::{Connection, RemoteReply, serve};
pub use constants::{MAX_CONNECTIONS, MAX_IN_FLIGHT, MAX_RECEIPT_WAIT, MAX_STREAMS};
pub use event_client::RemoteEvents;
pub use receipt_client::DeferredReceipt;
pub use yolo_capnp_codec::Value;
