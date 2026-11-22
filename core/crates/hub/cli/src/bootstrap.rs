use crate::{constants::*, options::Options};
use anyhow::{Result, anyhow};
use tokio::signal::unix::{SignalKind, signal};

pub(crate) async fn run(options: Options) -> Result<()> {
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut hangup = signal(SignalKind::hangup())?;
    let mut quit = signal(SignalKind::quit())?;
    let launch = yolo_hub_local_startup::Launch {
        state: options.state.expect(STATE_FIELD),
        host_home: options.host_home.expect(HOST_HOME_FIELD),
        host_executable: options.host_executable.expect(HOST_EXECUTABLE_FIELD),
        hub_executable: std::env::current_exe()?,
    };
    let deadline = tokio::time::Instant::now() + yolo_hub_local_startup::STARTUP_TIMEOUT;
    let state = launch.state.clone();
    let home = launch.host_home.clone();
    let result = tokio::select! {
        result = yolo_hub_local_startup::ensure_running(launch, deadline) => result,
        _ = terminate.recv() => Err(anyhow!(BOOTSTRAP_CANCELLED)),
        _ = interrupt.recv() => Err(anyhow!(BOOTSTRAP_CANCELLED)),
        _ = hangup.recv() => Err(anyhow!(BOOTSTRAP_CANCELLED)),
        _ = quit.recv() => Err(anyhow!(BOOTSTRAP_CANCELLED)),
    };
    yolo_process::wait_for_detached_termination_delivery(
        std::time::Instant::now() + BOOTSTRAP_RETIRE_TIMEOUT,
    )?;
    let document = yolo_hub_local_startup::BootstrapDocument::new(&state, &home, result?)?;
    use std::io::Write;
    std::io::stdout()
        .lock()
        .write_all(&yolo_capnp_codec::encode(&document)?)?;
    Ok(())
}
