use super::{
    FORCE_SIGNAL_REQUIRES_TERMINATION_DETAIL, ProcessGroupIdentity, ProcessIdentity, process_group,
};
use crate::process::{LiveProcessIdentity, signal_hook::run_pre_signal_hook};
use anyhow::{Result, ensure};

const INVALID_TARGET: &str = "foreground member requires a positive process birth identity";
const INVALID_SIGNAL: &str = "unsupported foreground member signal";

/// Delivers only to an exact member retained by a captured PTY group proof.
/// `true` means the kernel accepted the signal for that member, not that the
/// application acted on it. Group membership is rechecked before delivery;
/// the platform's atomic primitive binds process identity, not process group.
pub fn signal_process_group_member(
    group: ProcessGroupIdentity,
    expected: ProcessIdentity,
    signal: i32,
) -> Result<bool> {
    validate(expected, signal)?;
    let Some(current) = LiveProcessIdentity::capture(expected)? else {
        return Ok(false);
    };
    let Some(member) = group.members.iter().find(|member| **member == current) else {
        return Ok(false);
    };
    run_pre_signal_hook();
    if !member.try_matches()? || process_group(member.pid())? != Some(group.id) {
        return Ok(false);
    }
    // Never broaden to another group member when this exact sender returns false.
    group.authority.signal(member, signal)
}

fn validate(expected: ProcessIdentity, signal: i32) -> Result<()> {
    ensure!(
        expected.pid != u32::default()
            && i32::try_from(expected.pid).is_ok()
            && expected.start_time != u64::default(),
        INVALID_TARGET
    );
    ensure!(
        signal != libc::SIGKILL,
        FORCE_SIGNAL_REQUIRES_TERMINATION_DETAIL
    );
    // Match the foreground signal vocabulary accepted by parse_signal.
    ensure!(
        matches!(
            signal,
            libc::SIGINT
                | libc::SIGTERM
                | libc::SIGHUP
                | libc::SIGQUIT
                | libc::SIGTSTP
                | libc::SIGCONT
                | libc::SIGWINCH
        ),
        INVALID_SIGNAL
    );
    Ok(())
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/inventory/inventory_member_signal_tests.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/inventory_member_signal_tests.rs"]
mod unit_tests;
