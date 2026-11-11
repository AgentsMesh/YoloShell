use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityProcessIdentity {
    pub pid: u32,
    pub start_time: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthorityRuntimeIdentity {
    pub runtime_epoch: Uuid,
    pub worker: AuthorityProcessIdentity,
}
