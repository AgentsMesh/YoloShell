use super::{LiveProcessIdentity, ProcessIdentity};
use anyhow::Result;

pub(in crate::process) fn durable(
    identities: Vec<(LiveProcessIdentity, ProcessIdentity)>,
) -> Vec<ProcessIdentity> {
    identities
        .into_iter()
        .map(|(_live, durable)| durable)
        .collect()
}

pub(in crate::process) fn combine<T>(
    refresh: Option<anyhow::Error>,
    signals: Result<T>,
) -> Result<T> {
    match (refresh, signals) {
        (Some(refresh), Err(signals)) => Err(refresh.context(signals.to_string())),
        (Some(refresh), Ok(_delivered)) => Err(refresh),
        (None, signals) => signals,
    }
}
