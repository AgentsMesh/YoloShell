mod connection;
mod events;
mod service;

pub use connection::WorkerConnection;
pub(super) use service::serve;

const INVALID_RPC: &str = "invalid worker Cap'n Proto RPC request";
const CLOSED_RPC: &str = "worker Cap'n Proto event stream is closed";
