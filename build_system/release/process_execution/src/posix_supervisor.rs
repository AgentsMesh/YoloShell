use crate::constants::{
    DEFAULT_PROCESS_OUTPUT_BYTES, MAX_DRAIN_READS_PER_POLL, PROCESS_DRAIN_BUFFER_BYTES,
    PROCESS_POLL_INTERVAL,
};
use crate::posix_cleanup::terminate_process;
use crate::posix_spawn::{SpawnedProcess, spawn};
use crate::process::ProcessCommand;
use anyhow::{Context, Result, anyhow, bail};
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::process::Output;
use std::thread;
use std::time::Instant;

pub(crate) fn supervised_process(specification: &ProcessCommand) -> Result<Output> {
    let mut child = spawn(specification)?;
    let mut streams = Streams::with_limit(specification.output_limit);
    let outcome = run(specification, &mut child, &mut streams);
    let cleanup = terminate_process(&mut child, &mut streams);
    match (outcome, cleanup) {
        (Ok(()), Ok(status)) => Ok(Output {
            status,
            stdout: streams.stdout.bytes,
            stderr: streams.stderr.bytes,
        }),
        (Err(error), Ok(_)) => Err(error),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(error), Err(cleanup)) => Err(anyhow!(
            "{error:#}; spawned process cleanup failed: {cleanup:#}"
        )),
    }
}

fn run(
    specification: &ProcessCommand,
    child: &mut SpawnedProcess,
    streams: &mut Streams,
) -> Result<()> {
    child
        .write_input(specification.input.as_deref())
        .context("write spawned process input")?;
    let deadline = Instant::now() + specification.timeout;
    loop {
        streams.drain(child)?;
        child.refresh_custody()?;
        if streams.overflowed() {
            bail!(
                "{} output exceeded the bounded limit",
                specification.description
            );
        }
        if child.exited()? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            bail!("{} timed out", specification.description);
        }
        thread::sleep(
            PROCESS_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

pub(crate) struct Streams {
    pub(crate) stdout: DrainState,
    pub(crate) stderr: DrainState,
}

impl Streams {
    fn with_limit(max_bytes: usize) -> Self {
        Self {
            stdout: DrainState::with_limit(max_bytes),
            stderr: DrainState::with_limit(max_bytes),
        }
    }

    pub(crate) fn drain(&mut self, child: &mut SpawnedProcess) -> Result<()> {
        drain(&mut child.stdout, &mut self.stdout)?;
        drain(&mut child.stderr, &mut self.stderr)
    }

    pub(crate) fn complete(&self) -> bool {
        self.stdout.eof && self.stderr.eof
    }

    fn overflowed(&self) -> bool {
        self.stdout.overflowed || self.stderr.overflowed
    }
}

pub(crate) struct DrainState {
    pub(crate) bytes: Vec<u8>,
    max_bytes: usize,
    eof: bool,
    overflowed: bool,
}

impl Default for Streams {
    fn default() -> Self {
        Self::with_limit(DEFAULT_PROCESS_OUTPUT_BYTES)
    }
}

impl DrainState {
    fn with_limit(max_bytes: usize) -> Self {
        Self {
            bytes: Vec::new(),
            max_bytes,
            eof: false,
            overflowed: false,
        }
    }
}

fn drain(reader: &mut File, state: &mut DrainState) -> Result<()> {
    let mut buffer = [0_u8; PROCESS_DRAIN_BUFFER_BYTES];
    for _ in 0..MAX_DRAIN_READS_PER_POLL {
        match reader.read(&mut buffer) {
            Ok(0) => {
                state.eof = true;
                return Ok(());
            }
            Ok(count) => {
                let remaining = state.max_bytes.saturating_sub(state.bytes.len());
                state
                    .bytes
                    .extend_from_slice(&buffer[..count.min(remaining)]);
                state.overflowed |= count > remaining;
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error).context("drain child output"),
        }
    }
    Ok(())
}
