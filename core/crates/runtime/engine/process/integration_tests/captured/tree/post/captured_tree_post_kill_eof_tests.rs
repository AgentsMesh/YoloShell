use super::super::empty_eof_fixture::EmptyEofFixture;
use super::super::{
    FreezePoll, FrozenProducersGoneCommit, FrozenProducersGoneFinalization, FrozenProducersGonePoll,
};
use super::*;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::kernel_eof_authority::{KernelPtyEofIdentity, bind_scripted_kernel_eof};
use std::sync::Arc;

const REFREEZE_REQUIRED_DETAIL: &str =
    "released PTY uncertainty did not route through the kernel EOF transaction";
const PENDING_REQUIRED_DETAIL: &str =
    "post-inventory event debt bypassed exact post-kill reconciliation";
const FROZEN_REQUIRED_DETAIL: &str = "kernel EOF refreeze did not preserve frozen custody";
const GONE_REQUIRED_DETAIL: &str = "fresh kernel EOF did not prepare exact-empty settlement";
const REVALIDATION_REQUIRED_DETAIL: &str =
    "exact-empty settlement did not require final kernel EOF revalidation";
const FINALIZATION_REQUIRED_DETAIL: &str = "exact-empty settlement did not finalize";

#[test]
fn released_negative_scan_refreezes_until_fresh_kernel_eof_settles() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    let unresolved = fixture.tree.events.unresolved_fork_lineages_for_test();

    assert!(matches!(
        observe_rebuilt_exclusion(&mut fixture.tree),
        PostKillObservation::Refreeze
    ));
    assert_eq!(
        fixture.tree.events.unresolved_fork_lineages_for_test(),
        unresolved,
        "{REFREEZE_REQUIRED_DETAIL}"
    );

    let (freezing, _pty) = fixture.into_prepared_freezing_parts();
    let mut freezing = match freezing.poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{FROZEN_REQUIRED_DETAIL}"),
    };
    let ready = freezing.prepare_poll().unwrap();
    let frozen = match freezing.commit_poll(ready) {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{FROZEN_REQUIRED_DETAIL}"),
    };
    commit_fresh_kernel_eof(frozen);
}

#[test]
fn post_inventory_event_keeps_exact_empty_pending() {
    let mut fixture = EmptyEofFixture::exact_gone_source();
    fixture.tree.discovery_epoch = DiscoveryScanEpoch::Pending;
    let (_source, live) = fixture.register_source();
    fixture.tree.events.queue_test_fork(live);

    assert!(matches!(
        observe_rebuilt_exclusion(&mut fixture.tree),
        PostKillObservation::Pending
    ));
    assert!(
        fixture.tree.events.has_unresolved_fork(),
        "{PENDING_REQUIRED_DETAIL}"
    );
}

fn commit_fresh_kernel_eof(mut frozen: super::super::FrozenProcessTree) {
    let version = frozen.closure_version();
    let stale = KernelPtyEofIdentity::from_kernel_read();
    let (permit, _) = bind_scripted_kernel_eof(&version, &stale);
    let ready = prepare_gone(&mut frozen, permit);
    assert!(matches!(
        frozen.prepare_producers_gone_commit(&ready).unwrap(),
        FrozenProducersGoneCommit::FreshEofRequired
    ));
    let output = KernelPtyEofIdentity::from_kernel_read();
    assert!(!Arc::ptr_eq(&stale, &output));
    let (permit, validation) = bind_scripted_kernel_eof(&version, &output);
    let ready = prepare_gone(&mut frozen, permit);
    let FrozenProducersGoneCommit::ReopenSealed(seal) =
        frozen.prepare_producers_gone_commit(&ready).unwrap()
    else {
        panic!("{REVALIDATION_REQUIRED_DETAIL}");
    };
    let validated = validation.validate_scripted(&version, &output).unwrap();
    let validated = validated.bind_reopen_seal_for_test(&seal);
    let FrozenProducersGoneFinalization::Finalized(finalized) = frozen
        .finalize_producers_gone_commit(&ready, Some(validated))
        .unwrap()
    else {
        panic!("{FINALIZATION_REQUIRED_DETAIL}");
    };
    assert!(frozen.commit_producers_gone(finalized).is_ok());
}

fn prepare_gone(
    frozen: &mut super::super::FrozenProcessTree,
    permit: crate::process::kernel_eof_authority::KernelEofPermit,
) -> super::super::FrozenProducersGoneReady {
    match frozen
        .prepare_producers_gone_with_kernel_eof(permit)
        .unwrap()
    {
        FrozenProducersGonePoll::Gone(ready) => ready,
        _ => panic!("{GONE_REQUIRED_DETAIL}"),
    }
}
