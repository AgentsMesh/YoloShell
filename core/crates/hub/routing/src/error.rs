use crate::constants::ERROR_PREFIX;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoutingError {
    LocalPeer,
    LocalOrigin,
    EmptyRights,
    UnknownPeer,
    UnauthorizedOrigin,
    StaleConnection,
    StaleRevision,
    GenerationExhausted,
    EmptyPath,
    ForgedPeer,
    ForgedOrigin,
    CyclicPath,
    PathLimit,
    PeerLimit,
    OriginLimit,
    AgentLimit,
    RouteLimit,
    AdvertisementLimit,
    DuplicateRoute,
}

impl fmt::Display for RoutingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{ERROR_PREFIX}: {self:?}")
    }
}

impl std::error::Error for RoutingError {}
