use super::constants::*;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::worker::codex_hook) enum CodexRouterError {
    Scope,
    Runtime,
    Admission,
    Conflict,
    Lock,
    Capacity,
}

impl Display for CodexRouterError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Scope => ROUTER_SCOPE_ERROR,
            Self::Runtime => ROUTER_RUNTIME_ERROR,
            Self::Admission => ROUTER_ADMISSION_ERROR,
            Self::Conflict => ROUTER_CONFLICT_ERROR,
            Self::Lock => ROUTER_LOCK_ERROR,
            Self::Capacity => ROUTER_CAPACITY_ERROR,
        })
    }
}

impl std::error::Error for CodexRouterError {}
