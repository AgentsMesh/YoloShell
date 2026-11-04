use anyhow::{Context, Result, bail, ensure};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use yolo_host::failed_rearm_process_fixture::{
    Boundary, PreparedEvidence, crash, prepare, recover_and_verify,
};

#[path = "failed_rearm_process_constants.rs"]
mod constants;
use constants::*;

#[tokio::main]
async fn main() -> Result<()> {
    let mut arguments = std::env::args_os();
    let _executable = arguments.next();
    let mode = arguments
        .next()
        .and_then(|value| value.into_string().ok())
        .context(FIXTURE_MODE_MISSING)?;
    if mode == PROCESS_LIST_MODE {
        ensure!(arguments.next().is_none(), FIXTURE_ARGUMENT_INVALID);
        return write_process_listing();
    }
    let home = arguments
        .next()
        .map(PathBuf::from)
        .context(FIXTURE_HOME_MISSING)?;
    match mode.as_str() {
        PREPARE_MODE => {
            ensure!(arguments.next().is_none(), FIXTURE_ARGUMENT_INVALID);
            let evidence = prepare(&home).await?;
            std::io::stdout().write_all(evidence.encode()?.as_bytes())?;
        }
        CRASH_MODE | RECOVER_MODE => {
            let boundary = arguments
                .next()
                .and_then(|value| value.into_string().ok())
                .context(FIXTURE_BOUNDARY_MISSING)?;
            ensure!(arguments.next().is_none(), FIXTURE_ARGUMENT_INVALID);
            run_existing(&mode, &home, Boundary::parse(&boundary)?).await?;
        }
        DRAIN_STRESS_MODE => {
            ensure!(arguments.next().is_none(), FIXTURE_ARGUMENT_INVALID);
            let output = vec![DRAIN_STRESS_BYTE; DRAIN_STRESS_BYTES];
            std::io::stdout().write_all(&output)?;
        }
        CAPTURE_STRESS_MODE => {
            ensure!(arguments.next().is_none(), FIXTURE_ARGUMENT_INVALID);
            std::thread::sleep(CAPTURE_STRESS_DURATION);
        }
        _ => bail!(FIXTURE_ARGUMENT_INVALID),
    }
    Ok(())
}

fn write_process_listing() -> Result<()> {
    let output = Command::new(PROCESS_LIST_EXECUTABLE)
        .args(PROCESS_LIST_ARGUMENTS.iter().copied())
        .output()
        .context(PROCESS_LIST_FAILURE)?;
    ensure!(output.status.success(), PROCESS_LIST_FAILURE);
    ensure!(output.stderr.is_empty(), PROCESS_LIST_FAILURE);
    std::io::stdout().write_all(&output.stdout)?;
    Ok(())
}

async fn run_existing(mode: &str, home: &std::path::Path, boundary: Boundary) -> Result<()> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context(FIXTURE_INPUT_CONTEXT)?;
    let evidence = PreparedEvidence::decode(&input)?;
    match mode {
        CRASH_MODE => crash(home, boundary, &evidence).await,
        RECOVER_MODE => recover_and_verify(home, boundary, &evidence).await,
        _ => bail!(FIXTURE_ARGUMENT_INVALID),
    }
}
