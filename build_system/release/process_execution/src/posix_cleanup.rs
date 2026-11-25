use crate::constants::{PROCESS_DRAIN_GRACE, PROCESS_POLL_INTERVAL};
use crate::posix_spawn::SpawnedProcess;
use crate::posix_supervisor::Streams;
use crate::process_group_members::has_other_members;
use anyhow::{Context, Result, anyhow};
use std::process::ExitStatus;
use std::thread;
use std::time::{Duration, Instant};

const GRACEFUL_SIGNAL_CONTEXT: &str = "signal spawned process custody for graceful termination";
const DISCOVERY_SETTLEMENT_CONTEXT: &str = "settle discovery process custody";
const FORCE_SIGNAL_CONTEXT: &str = "force spawned process custody";
const FORCE_WAIT_CONTEXT: &str = "wait for forced process custody";
const DIRECT_EXIT_CONTEXT: &str = "observe direct child after forced cleanup";
const DIRECT_SURVIVAL_DETAIL: &str = "direct child survived SIGKILL grace";
const GROUP_INSPECTION_CONTEXT: &str = "inspect spawned process group after forced cleanup";
const GROUP_SURVIVAL_DETAIL: &str = "spawned process group survived SIGKILL grace";
const REAP_CONTEXT: &str = "reap spawned process";
const REAP_SKIPPED_DETAIL: &str = "spawned process could not be safely reaped";
const DRAIN_CONTEXT: &str = "drain spawned process pipes after cleanup";
const PIPE_EOF_DETAIL: &str = "spawned process pipes did not reach EOF";
const FAILURE_SEPARATOR: &str = "; ";

pub(crate) fn terminate_process(
    child: &mut SpawnedProcess,
    streams: &mut Streams,
) -> Result<ExitStatus> {
    let mut failures = Failures::default();
    failures.record(child.signal_custody(libc::SIGTERM), GRACEFUL_SIGNAL_CONTEXT);
    let discovery_settled =
        failures.capture(child.settle_discovery(), DISCOVERY_SETTLEMENT_CONTEXT);
    failures.record(child.signal_custody(libc::SIGKILL), FORCE_SIGNAL_CONTEXT);
    failures.record(
        wait_for_custody(child, streams, PROCESS_DRAIN_GRACE),
        FORCE_WAIT_CONTEXT,
    );
    let direct_exited = failures
        .capture(child.exited(), DIRECT_EXIT_CONTEXT)
        .unwrap_or(false);
    if !direct_exited {
        failures.push(DIRECT_SURVIVAL_DETAIL);
    }
    let group_empty = failures
        .capture(
            has_other_members(child.pid, child.pid),
            GROUP_INSPECTION_CONTEXT,
        )
        .is_some_and(|has_members| !has_members);
    if group_empty {
        child.mark_group_settled();
    } else {
        failures.push(GROUP_SURVIVAL_DETAIL);
    }
    let status = if direct_exited && group_empty {
        failures.capture(child.wait(), REAP_CONTEXT)
    } else {
        failures.push(REAP_SKIPPED_DETAIL);
        None
    };
    failures.record(
        drain_for(child, streams, PROCESS_DRAIN_GRACE),
        DRAIN_CONTEXT,
    );
    if !streams.complete() {
        failures.push(PIPE_EOF_DETAIL);
    }
    if discovery_settled.is_some() && group_empty && status.is_some() && streams.complete() {
        child.settle();
    }
    failures.finish(status)
}

fn wait_for_custody(
    child: &mut SpawnedProcess,
    streams: &mut Streams,
    duration: Duration,
) -> Result<()> {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        streams.drain(child)?;
        if child.exited()? && !has_other_members(child.pid, child.pid)? {
            break;
        }
        pause(deadline);
    }
    Ok(())
}

fn drain_for(child: &mut SpawnedProcess, streams: &mut Streams, duration: Duration) -> Result<()> {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline && !streams.complete() {
        streams.drain(child)?;
        if !streams.complete() {
            pause(deadline);
        }
    }
    Ok(())
}

fn pause(deadline: Instant) {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if !remaining.is_zero() {
        thread::sleep(PROCESS_POLL_INTERVAL.min(remaining));
    }
}

#[derive(Default)]
struct Failures(Vec<String>);

impl Failures {
    fn record(&mut self, result: Result<()>, context: &'static str) {
        self.capture(result, context);
    }

    fn capture<T>(&mut self, result: Result<T>, context: &'static str) -> Option<T> {
        match result.with_context(|| context) {
            Ok(value) => Some(value),
            Err(error) => {
                self.0.push(format!("{error:#}"));
                None
            }
        }
    }

    fn push(&mut self, detail: &'static str) {
        self.0.push(detail.to_owned());
    }

    fn finish(self, status: Option<ExitStatus>) -> Result<ExitStatus> {
        match (self.0.is_empty(), status) {
            (true, Some(status)) => Ok(status),
            (true, None) => Err(anyhow!(REAP_SKIPPED_DETAIL)),
            (false, _) => Err(anyhow!(self.0.join(FAILURE_SEPARATOR))),
        }
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/cleanup_failures.rs"]
mod unit_tests;
