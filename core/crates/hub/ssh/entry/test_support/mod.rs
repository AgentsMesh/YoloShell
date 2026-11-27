mod constants;
mod fixture;
mod inner;
pub mod keys;
mod outer;
pub use constants::*;
pub use fixture::Fixture;
pub use inner::Service;

use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::Instant;
pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}

#[derive(Default)]
pub struct Counts {
    pub connections: std::sync::atomic::AtomicUsize,
    pub forwards: std::sync::atomic::AtomicUsize,
    pub mutations: std::sync::atomic::AtomicUsize,
    pub closed: std::sync::atomic::AtomicUsize,
}
