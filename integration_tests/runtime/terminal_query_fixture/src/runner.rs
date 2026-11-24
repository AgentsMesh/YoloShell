use crate::{Command, Report, constants::*};
use anyhow::{Result, ensure};
use std::{io::ErrorKind, path::Path};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time::{Instant, interval, sleep_until},
};
use yolo_hub_terminal_io::{Descriptor, RawTerminal};

pub async fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    ensure!(
        args.len() == ARGUMENTS,
        "{ERROR}: phase=arguments expected_count={ARGUMENTS} actual_count={}",
        args.len()
    );
    let root = Path::new(&args[DIRECTORY_ARGUMENT]);
    ensure!(
        root.is_absolute() && root.is_dir(),
        "{ERROR}: phase=directory expected=absolute_directory absolute={} directory={}",
        root.is_absolute(),
        root.is_dir()
    );
    let _raw = RawTerminal::enable(libc::STDIN_FILENO)?.ok_or_else(|| {
        anyhow::anyhow!("{ERROR}: phase=raw_terminal expected=terminal actual=nonterminal")
    })?;
    let (mut input, mut output) = Descriptor::pair(libc::STDIN_FILENO, libc::STDOUT_FILENO)?;
    std::fs::write(root.join(READY_FILE), READY_VALUE)?;
    let mut timer = interval(POLL);
    let lifetime = Instant::now() + LIFETIME;
    let mut settle = lifetime;
    let mut active = false;
    let mut sequence = u64::default();
    let mut received = Vec::new();
    let mut buffer = [0; READ_BYTES];
    loop {
        tokio::select! {
            biased;
            count = input.read(&mut buffer) => {
                let count = count?;
                if count == 0 { return Ok(()); }
                ensure!(received.len() + count <= MAX_RECEIVED_BYTES,
                    "{ERROR}: phase=input sequence={sequence} expected_max={MAX_RECEIVED_BYTES} actual={}",
                    received.len() + count);
                received.extend_from_slice(&buffer[..count]);
            },
            _ = sleep_until(lifetime) => anyhow::bail!(
                "{ERROR}: phase=lifetime expected_completion_before={LIFETIME:?} sequence={sequence} active={active} received_bytes={}",
                received.len()),
            _ = sleep_until(settle), if active => {
                let report = Report { sequence, received: received.clone() };
                std::fs::write(root.join(TEMP_REPORT_FILE), serde_json::to_vec(&report)?)?;
                std::fs::rename(root.join(TEMP_REPORT_FILE), root.join(REPORT_FILE))?;
                active = false;
            },
            _ = timer.tick(), if !active => {
                if let Some(command) = command(root, sequence)? {
                    output.write_all(&command.output).await?;
                    output.flush().await?;
                    sequence = command.sequence;
                    settle = Instant::now() + std::time::Duration::from_millis(command.settle_ms);
                    active = true;
                }
            },
        }
    }
}

fn command(root: &Path, previous: u64) -> Result<Option<Command>> {
    let path = root.join(COMMAND_FILE);
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let bytes = file.metadata()?.len();
    ensure!(
        bytes <= MAX_COMMAND_BYTES,
        "{ERROR}: phase=command_file expected_max={MAX_COMMAND_BYTES} actual={bytes}"
    );
    let command: Command = serde_json::from_reader(file)?;
    if command.sequence == previous {
        return Ok(None);
    }
    let expected = previous.checked_add(SEQUENCE_STEP).ok_or_else(|| {
        anyhow::anyhow!("{ERROR}: phase=sequence expected=successor actual_previous={previous}")
    })?;
    ensure!(
        command.sequence == expected,
        "{ERROR}: phase=sequence expected={expected} actual={}",
        command.sequence
    );
    ensure!(
        command.output.len() <= MAX_OUTPUT_BYTES,
        "{ERROR}: phase=output expected_max={MAX_OUTPUT_BYTES} actual={}",
        command.output.len()
    );
    ensure!(
        command.settle_ms > 0 && command.settle_ms <= MAX_SETTLE_MS,
        "{ERROR}: phase=settle expected_range=1..={MAX_SETTLE_MS} actual={}",
        command.settle_ms
    );
    Ok(Some(command))
}
