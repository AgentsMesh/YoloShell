use super::*;
use crate::process::inventory_test_support::TestProcess;

const TEST_SIGNAL: i32 = libc::SIGCONT;
const NOOP_TARGET_INDEX: usize = 1;
const FAILURE_TARGET_INDEX: usize = 2;
const TARGET_COUNT: usize = 5;
const INSTALLED_HOOK_COUNT: usize = 2;
const FIRST_DELIVERY: usize = 0;
const SECOND_DELIVERY: usize = 1;
const DELIVERY_INCREMENT: usize = 1;
const SIGNAL_FAILURE: &str = "injected matrix signal failure";
const REFRESH_FAILURE: &str = "injected matrix refresh failure";
const REPLACEMENT_GENERATION_STEP: u64 = 1;
const REPLACEMENT_AUTHORIZED_DETAIL: &str = "replacement generation received exact authority";
const LIVE_NOOP_AUTHORIZED_DETAIL: &str = "live no-op received exact authority";
const ABSENT_REJECTED_DETAIL: &str = "confirmed absent generation was not resolved";

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn one_delivery_instance_covers_delivery_noop_and_repeated_failure() {
    let process = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    let targets = std::iter::repeat_n((live, process.identity), TARGET_COUNT).collect::<Vec<_>>();
    let _noop = install_test_signal_noop(TEST_SIGNAL, NOOP_TARGET_INDEX);
    let _failure = install_test_signal_failure(TEST_SIGNAL, FAILURE_TARGET_INDEX, SIGNAL_FAILURE);
    let mut deliveries = FIRST_DELIVERY;

    let error = signal_with(&targets, TEST_SIGNAL, |_identity, _signal| {
        let delivery = deliveries;
        deliveries += DELIVERY_INCREMENT;
        match delivery {
            FIRST_DELIVERY => Ok(true),
            SECOND_DELIVERY => Ok(false),
            _ => anyhow::bail!(SIGNAL_FAILURE),
        }
    })
    .unwrap_err();

    assert!(format!("{error:#}").contains(SIGNAL_FAILURE));
    assert_eq!(deliveries, TARGET_COUNT - INSTALLED_HOOK_COUNT);
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn combine_preserves_refresh_authority_for_success_and_signal_failure() {
    let refresh = || anyhow::anyhow!(REFRESH_FAILURE);
    let signals = || anyhow::anyhow!(SIGNAL_FAILURE);

    let failed: Result<u8> = combine(Some(refresh()), Err(signals()));
    assert!(format!("{:#}", failed.unwrap_err()).contains(REFRESH_FAILURE));

    let delivered: Result<u8> = combine(Some(refresh()), Ok(u8::MAX));
    assert!(format!("{:#}", delivered.unwrap_err()).contains(REFRESH_FAILURE));
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn exact_sigkill_resolves_a_confirmed_absent_generation() {
    let mut process = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    let mut tree = crate::process::CapturedProcessTree::capture(process.identity).unwrap();
    tree.authorize_destructive_for_test();
    process.terminate();

    let resolution = signal_sigkill(
        tree.destructive_authority().unwrap(),
        &[(live, process.identity)],
    )
    .unwrap();

    assert!(resolution.is_some(), "{ABSENT_REJECTED_DETAIL}");
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn exact_sigkill_rejects_a_replacement_generation() {
    let process = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    let replacement =
        live.with_generation(live.lineage_id().wrapping_add(REPLACEMENT_GENERATION_STEP));
    let mut tree = crate::process::CapturedProcessTree::capture(process.identity).unwrap();
    tree.authorize_destructive_for_test();

    let resolution = signal_sigkill(
        tree.destructive_authority().unwrap(),
        &[(replacement, process.identity)],
    )
    .unwrap();

    assert!(resolution.is_none(), "{REPLACEMENT_AUTHORIZED_DETAIL}");
    assert!(process.identity.matches());
}

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn exact_sigkill_rejects_a_live_noop() {
    let process = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    let mut tree = crate::process::CapturedProcessTree::capture(process.identity).unwrap();
    tree.authorize_destructive_for_test();
    let _noop = install_test_signal_noop(libc::SIGKILL, usize::default());

    let resolution = signal_sigkill(
        tree.destructive_authority().unwrap(),
        &[(live, process.identity)],
    )
    .unwrap();

    assert!(resolution.is_none(), "{LIVE_NOOP_AUTHORIZED_DETAIL}");
    assert!(process.identity.matches());
}
