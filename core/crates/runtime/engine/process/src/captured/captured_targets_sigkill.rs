use super::{LiveProcessIdentity, ProcessIdentity};
use crate::process::signal_authority::{DestructiveSessionAuthority, ExactSignalOutcome};
use anyhow::{Context, Result};
use std::collections::HashSet;

pub(in crate::process) struct ExactSigkillResolution {
    target_lineages: HashSet<u64>,
}

impl ExactSigkillResolution {
    pub(in crate::process) fn from_targets(
        targets: &[(LiveProcessIdentity, ProcessIdentity)],
    ) -> Self {
        Self {
            target_lineages: targets
                .iter()
                .map(|(identity, _durable)| identity.lineage_id())
                .collect(),
        }
    }

    pub(in crate::process) fn target_lineages(&self) -> &HashSet<u64> {
        &self.target_lineages
    }
}

pub(in crate::process) fn signal(
    authority: &DestructiveSessionAuthority,
    targets: &[(LiveProcessIdentity, ProcessIdentity)],
) -> Result<Option<ExactSigkillResolution>> {
    if targets.is_empty() {
        return Ok(None);
    }
    crate::process::controller_boundary::ensure_signal_targets_are_external(targets)?;
    let mut failure = None;
    let mut resolved = !targets.is_empty();
    for (target_index, (identity, durable)) in targets.iter().enumerate() {
        let result = signal_target(authority, identity, target_index)
            .with_context(|| format!("{}: {durable:?}", super::SIGNAL_IDENTITY_CONTEXT));
        match result {
            Ok(outcome) => resolved &= outcome.authorizes_resolution(),
            Err(error) if failure.is_none() => failure = Some(error),
            Err(_) => {}
        }
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(resolved.then(|| ExactSigkillResolution::from_targets(targets))),
    }
}

fn signal_target(
    authority: &DestructiveSessionAuthority,
    identity: &LiveProcessIdentity,
    target_index: usize,
) -> Result<ExactSignalOutcome> {
    let mut outcome = None;
    let delivered = super::signal_target(
        identity,
        libc::SIGKILL,
        target_index,
        &mut |identity, _signal| {
            let current = authority.signal_exact_sigkill(identity)?;
            outcome = Some(current);
            Ok(current == ExactSignalOutcome::Delivered)
        },
    )?;
    Ok(outcome.unwrap_or(if delivered {
        ExactSignalOutcome::Delivered
    } else {
        ExactSignalOutcome::Undelivered
    }))
}
