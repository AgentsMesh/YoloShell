use super::empty_eof_fixture::EmptyEofFixture;
use super::{FreezePoll, FrozenProducersGoneCommit, FrozenProducersGonePoll};
use crate::process::LiveProcessIdentity;
use crate::process::inventory_test_support::TestProcess;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

const SLAVE_OPEN_FLAGS: libc::c_int =
    libc::O_NOCTTY | libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW;
const EXPECTED_PENDING_DETAIL: &str = "empty kernel EOF freeze skipped confirmation";
const EXPECTED_FROZEN_DETAIL: &str = "empty kernel EOF freeze stayed pending";
const EXPECTED_GONE_DETAIL: &str = "empty kernel EOF closure did not prepare gone";
const LATE_SOURCE_ACCEPTED_DETAIL: &str = "late source did not revoke producers-gone authority";
const REOPEN_ALLOWED_DETAIL: &str = "PTY reopen remained possible during empty refreeze";

#[test]
fn empty_refreeze_observes_late_source_only_after_reopen_is_sealed() {
    let (freezing, pty) = EmptyEofFixture::exact_gone_source().into_prepared_freezing_parts();
    let path = pty.master.tty_name().unwrap();
    let mut freezing = match freezing.poll().unwrap() {
        FreezePoll::Pending(freezing) => freezing,
        FreezePoll::Frozen(_) => panic!("{EXPECTED_PENDING_DETAIL}"),
    };
    let ready = freezing
        .prepare_poll()
        .unwrap()
        .authorize_kernel_eof_for_test();
    let mut frozen = match freezing.commit_poll(ready) {
        FreezePoll::Frozen(frozen) => frozen,
        FreezePoll::Pending(_) => panic!("{EXPECTED_FROZEN_DETAIL}"),
    };
    let permit = frozen.kernel_eof_permit_for_test();
    let ready = match frozen
        .prepare_producers_gone_with_kernel_eof(permit)
        .unwrap()
    {
        FrozenProducersGonePoll::Gone(ready) => ready,
        _ => panic!("{EXPECTED_GONE_DETAIL}"),
    };
    let source = TestProcess::spawn();
    let live = LiveProcessIdentity::capture(source.identity)
        .unwrap()
        .unwrap();
    let _ = frozen.tree.events.reconcile_inventory([live]);
    frozen.tree.events.queue_test_fork(live);

    let prepared = frozen.prepare_producers_gone_commit(&ready).unwrap();

    assert!(
        matches!(prepared, FrozenProducersGoneCommit::FreshEofRequired),
        "{LATE_SOURCE_ACCEPTED_DETAIL}"
    );
    let reopened = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(SLAVE_OPEN_FLAGS)
        .open(path);
    assert!(reopened.is_err(), "{REOPEN_ALLOWED_DETAIL}");
}
