use super::signal_authority::DestructiveSessionAuthority;
use super::{LiveProcessIdentity, ProcessIdentity};
use anyhow::{Context, Result};

#[path = "captured_targets_helpers.rs"]
mod helpers;
#[path = "captured_targets_sigkill.rs"]
mod sigkill;
#[cfg(all(target_os = "macos", any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests")))]
#[path = "../../test_support/captured/captured_targets_hook.rs"]
pub(crate) mod test_hook;
pub(super) use helpers::{combine, durable};
pub(super) use sigkill::{ExactSigkillResolution, signal as signal_sigkill};

const SIGNAL_IDENTITY_CONTEXT: &str = "signal captured process-tree identity";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
include!("../../test_support/captured/captured_target_faults.rs");

pub(super) fn sorted(
    root: &LiveProcessIdentity,
    identities: impl Iterator<Item = (LiveProcessIdentity, ProcessIdentity)>,
) -> Vec<(LiveProcessIdentity, ProcessIdentity)> {
    let mut identities = identities.collect::<Vec<_>>();
    identities.sort_by_key(|(live, durable)| {
        (
            live == root,
            live.pid(),
            durable.start_time,
            live.lineage_id(),
        )
    });
    identities
}

pub(super) fn signal(
    authority: &DestructiveSessionAuthority,
    identities: &[(LiveProcessIdentity, ProcessIdentity)],
    signal: i32,
) -> Result<Vec<(LiveProcessIdentity, ProcessIdentity)>> {
    signal_with(identities, signal, |identity, signal| {
        authority.signal(identity, signal)
    })
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn signal_for_test(
    identities: &[(LiveProcessIdentity, ProcessIdentity)],
    signal: i32,
) -> Result<Vec<(LiveProcessIdentity, ProcessIdentity)>> {
    signal_with(identities, signal, LiveProcessIdentity::signal_for_test)
}

fn signal_with(
    identities: &[(LiveProcessIdentity, ProcessIdentity)],
    signal: i32,
    mut deliver: impl FnMut(&LiveProcessIdentity, i32) -> Result<bool>,
) -> Result<Vec<(LiveProcessIdentity, ProcessIdentity)>> {
    if identities.is_empty() {
        return Ok(Vec::new());
    }
    super::controller_boundary::ensure_signal_targets_are_external(identities)?;
    let mut failure = None;
    let mut delivered = Vec::new();
    for (target_index, (identity, durable)) in identities.iter().cloned().enumerate() {
        let result = signal_target(&identity, signal, target_index, &mut deliver)
            .with_context(|| format!("{SIGNAL_IDENTITY_CONTEXT}: {durable:?}"));
        match result {
            Ok(true) => delivered.push((identity, durable)),
            Ok(false) => {}
            Err(error) if failure.is_none() => failure = Some(error),
            Err(_) => {}
        }
    }
    failure.map_or(Ok(delivered), Err)
}

fn signal_target(
    identity: &LiveProcessIdentity,
    signal: i32,
    target_index: usize,
    deliver: &mut impl FnMut(&LiveProcessIdentity, i32) -> Result<bool>,
) -> Result<bool> {
    #[cfg(all(target_os = "macos", any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests")))]
    test_hook::run(signal, target_index);
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    if let Some(detail) = TEST_SIGNAL_FAILURE.with(|slot| {
        let mut failure = slot.borrow_mut();
        let matches = failure.as_ref().is_some_and(|failure| {
            failure.signal == signal && failure.target_index == target_index
        });
        matches.then(|| failure.take().unwrap().detail)
    }) {
        anyhow::bail!(detail);
    }
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    if TEST_SIGNAL_NOOP.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|noop| noop.signal == signal && noop.target_index == target_index)
    }) {
        return Ok(false);
    }
    #[cfg(not(test))]
    let _ = target_index;
    deliver(identity, signal)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/captured_targets_instance_matrix.rs"]
mod instance_matrix;
