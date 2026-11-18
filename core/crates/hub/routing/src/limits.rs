use std::num::NonZeroUsize;

/// Limits are supplied by the deployment policy, never by an advertisement.
#[derive(Clone, Copy, Debug)]
pub struct RoutingLimits {
    pub max_path_hubs: NonZeroUsize,
    pub max_peers: NonZeroUsize,
    pub max_origins_per_peer: NonZeroUsize,
    pub max_known_agents: NonZeroUsize,
    pub max_routes_per_agent: NonZeroUsize,
    pub max_advertisements_per_peer: NonZeroUsize,
}
