pub mod constants;
mod runner;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub sequence: u64,
    pub output: Vec<u8>,
    pub settle_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub sequence: u64,
    pub received: Vec<u8>,
}

pub use runner::run;
