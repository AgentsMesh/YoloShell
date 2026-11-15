use super::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::namespace_lease::NamespaceLease;
use crate::paths::identity::FileIdentity;
use anyhow::{Result, ensure};
use std::path::Path;

pub(super) const AUTHORITY_CHANGED: &str = "Codex binding authority changed during publication";

pub(super) fn validate_progression(
    current: Option<&CodexBindingAuthority>,
    next: &CodexBindingAuthority,
) -> Result<()> {
    let Some(current) = current else {
        return Ok(());
    };
    let left = current.version;
    let right = next.version;
    if current.status == CodexAuthorityStatus::Revoked {
        ensure!(
            right.route_revision > left.route_revision,
            AUTHORITY_CHANGED
        );
    }
    let same_version = right.route_revision == left.route_revision
        && right.invocation_id == left.invocation_id
        && right.generation == left.generation
        && right.authority_revision == left.authority_revision
        && right.sequence == left.sequence;
    if same_version {
        ensure!(
            next.same_runtime_provider(current)
                && next.same_resume_source(current)
                && ((next == current)
                    || next.is_canonical_terminal_successor_of(current)
                    || next.is_canonical_reactivation_of(current)
                    || current.revoked().as_ref().ok() == Some(next)),
            AUTHORITY_CHANGED
        );
        return Ok(());
    }
    let same_route =
        right.route_revision == left.route_revision && right.invocation_id == left.invocation_id;
    let same_generation = same_route && right.generation == left.generation;
    ensure!(
        !same_route || next.same_runtime_provider(current),
        AUTHORITY_CHANGED
    );
    ensure!(
        !same_generation || next.resume_progression_is_valid(current),
        AUTHORITY_CHANGED
    );
    ensure!(
        !same_generation
            || right.authority_revision != left.authority_revision
            || right.sequence == left.sequence,
        AUTHORITY_CHANGED
    );
    let revision_advances =
        right.authority_revision > left.authority_revision && right.sequence >= left.sequence;
    ensure!(
        right.route_revision > left.route_revision
            || (same_route
                && (right.generation > left.generation || (same_generation && revision_advances))),
        AUTHORITY_CHANGED
    );
    Ok(())
}

pub(super) fn read_with_identity<T: serde::de::DeserializeOwned>(
    namespace: &NamespaceLease,
    path: &Path,
) -> Result<Option<(T, FileIdentity)>> {
    match namespace.read_json_with_identity(path) {
        Ok(value) => Ok(Some(value)),
        Err(error) if missing(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(super) fn missing(error: &anyhow::Error) -> bool {
    error.chain().any(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io| io.kind() == std::io::ErrorKind::NotFound)
    })
}
