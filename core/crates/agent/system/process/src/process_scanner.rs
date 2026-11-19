use super::FencedDescendants;
use crate::process::ProcessIdentity;
use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

#[derive(Debug)]
pub struct ProcessScanObservation {
    pub descendants: FencedDescendants,
    pub started_at_ms: i64,
    pub completed_at_ms: i64,
}

pub type ProcessScanFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProcessScanObservation>> + Send + 'a>>;

pub trait ProcessScanner: Send + Sync {
    fn scan<'a>(&'a self, root: ProcessIdentity, deadline: Instant) -> ProcessScanFuture<'a>;
}
