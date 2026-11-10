const TEST_FORCE_TIMEOUT: Duration =
    FORCE_TERMINATION_TIMEOUT.saturating_add(FORCE_TERMINATION_CONFIRM_TIMEOUT);
const TEST_FORCE_TIMEOUT_DETAIL: &str = "typed test process-tree force transaction timed out";
const TEST_FORCE_IDENTITY_DETAIL: &str = "typed test force identity does not match its root";
const TEST_FORCE_SELF_DETAIL: &str = "typed test force cannot target the current process";

fn force_test_process_tree(
    root: process::ProcessIdentity,
    live: &process::LiveProcessIdentity,
) -> Result<()> {
    anyhow::ensure!(root.pid != std::process::id(), TEST_FORCE_SELF_DETAIL);
    anyhow::ensure!(live.pid() == root.pid, TEST_FORCE_IDENTITY_DETAIL);
    if !live.try_matches()? {
        return Ok(());
    }
    let tree = match process::CapturedProcessTree::capture(root) {
        Ok(tree) => tree,
        Err(error) => {
            if live.try_matches()? {
                return Err(error);
            }
            return Ok(());
        }
    };
    anyhow::ensure!(tree.live_root() == *live, TEST_FORCE_IDENTITY_DETAIL);
    let deadline = Instant::now() + TEST_FORCE_TIMEOUT;
    let frozen = drive_test_freeze(tree.begin_force(), deadline)?;
    let pending = drive_test_kill(frozen, deadline)?;
    drive_test_gone(pending, deadline)
}

fn drive_test_freeze(
    mut freezing: process::FreezingProcessTree,
    deadline: Instant,
) -> Result<process::FrozenProcessTree> {
    loop {
        match freezing.poll() {
            Ok(process::FreezePoll::Pending(next)) => freezing = next,
            Ok(process::FreezePoll::Frozen(frozen)) => return Ok(frozen),
            Err(error) => freezing = error.into_state(),
        }
        wait_for_test_force_progress(deadline)?;
    }
}

fn drive_test_kill(
    mut frozen: process::FrozenProcessTree,
    deadline: Instant,
) -> Result<process::KillPendingProcessTree> {
    loop {
        let version = frozen.closure_version();
        match frozen.kill_after_output_fence(version) {
            Ok(process::KillCommitPoll::Pending(pending)) => return Ok(pending),
            Ok(process::KillCommitPoll::Refreeze(freezing)) => {
                frozen = drive_test_freeze(freezing, deadline)?;
            }
            Err(error) => frozen = error.into_state(),
        }
        wait_for_test_force_progress(deadline)?;
    }
}

fn drive_test_gone(mut pending: process::KillPendingProcessTree, deadline: Instant) -> Result<()> {
    loop {
        match pending.poll() {
            Ok(process::KillPoll::Pending(next)) => pending = next,
            Ok(process::KillPoll::Gone(_proof)) => return Ok(()),
            Ok(process::KillPoll::Refreeze(freezing)) => {
                let frozen = drive_test_freeze(freezing, deadline)?;
                pending = drive_test_kill(frozen, deadline)?;
            }
            Err(error) => pending = error.into_state(),
        }
        wait_for_test_force_progress(deadline)?;
    }
}

fn wait_for_test_force_progress(deadline: Instant) -> Result<()> {
    anyhow::ensure!(Instant::now() < deadline, TEST_FORCE_TIMEOUT_DETAIL);
    std::thread::sleep(GRACEFUL_TERMINATION_POLL_INTERVAL);
    Ok(())
}

pub(super) fn fixture_root_identity(fixture: &Fixture) -> process::ProcessIdentity {
    let record = lock_recover(&fixture.state.record);
    process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    }
}
