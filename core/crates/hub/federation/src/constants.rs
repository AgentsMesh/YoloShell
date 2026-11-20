use std::time::Duration;
pub const POLL_INTERVAL: Duration = Duration::from_secs(2);
/// Catalogue-heartbeat refresh; Host change notifications refresh sooner.
pub const PUBLISH_HEARTBEAT: Duration = Duration::from_secs(30);
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
pub const STOP_TIMEOUT: Duration = Duration::from_secs(5);
pub const GRANT_LIFETIME_MS: u64 = 300_000;
pub(crate) const AGENT_DIRECTORY: &str = "agents";
pub(crate) const PUBLICATION_DIRECTORY: &str = "publication";
pub(crate) const IDENTITY_MISMATCH: &str = "federation and local backend identities differ";
pub(crate) const FILE_ROOTS_UNPREPARED: &str = "Hub file roots were not preflighted";
pub(crate) const CLOSED: &str = "federation is stopping";
pub(crate) const LOCAL_MANAGEMENT_STREAM_DETAIL: &str =
    "local management command cannot open a peer terminal stream";
pub(crate) const DUPLICATE_LINK: &str = "preferred peer connection is already active";
pub(crate) const INVALID_PEER: &str = "peer is not configured for this Hub";
pub(crate) const INVALID_DISCOVERY: &str = "peer discovery response does not match its request";
pub(crate) const EXHAUSTED: &str = "peer revision or clock is exhausted";
pub(crate) const PUBLICATION_FAILED: &str = "local Hub observation refresh failed";
pub(crate) const OBSERVATION_CACHE_UNAVAILABLE: &str =
    "offline Hub observations could not be retained; live services remain available";
pub(crate) const OBSERVATION_WORKER: &str = "hub-observations";
pub(crate) const OBSERVATION_WORKER_BUSY: &str =
    "an observation cache worker for this Hub is still running; cache unavailable";
pub(crate) const OBSERVATION_LOAD_TIMEOUT: Duration = Duration::from_secs(5);
pub(crate) const OBSERVATION_WORKER_STALLED: &str =
    "retired observation cache worker remains blocked; storage custody is retained";

pub(crate) const NO_DIALER: &str = "this Hub has no SSH peer dialer";
