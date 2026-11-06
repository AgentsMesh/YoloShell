use crate::{constants::*, helpers::Helpers, receipt};
use anyhow::{Context, Result, ensure};
use std::process::{Command, Stdio};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::{Instant, timeout_at},
};
use yolo_hub_local_startup::{ConnectionDocument, MAX_READY_BYTES};
use yolo_launch_contract::{HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT, ProcessRole};
use yolo_local_profile::LocalProfile;
use yolo_process::ScopedWaitableChild;

pub(crate) async fn run(
    profile: &LocalProfile,
    helpers: &Helpers,
    deadline: Instant,
) -> Result<ConnectionDocument> {
    let mut command = Command::new(&helpers.executable);
    command
        .args(ProcessRole::Hub.arguments())
        .arg(BOOTSTRAP_ARGUMENT)
        .arg(HUB_STATE_ARGUMENT)
        .arg(&profile.state)
        .arg(HOST_HOME_ARGUMENT)
        .arg(&profile.host_home)
        .arg(HOST_EXECUTABLE_ARGUMENT)
        .arg(&helpers.executable)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    // Drop retires only this exact helper. Existing startup guards settle its
    // uncommitted children on EOF; committed services retain their own custody.
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(&mut command, deadline.into_std())?;
    let stdout =
        tokio::process::ChildStdout::from_std(child.take_stdout().context(OUTPUT_MISSING)?)?;
    let (bytes, ()) = timeout_at(deadline, async {
        let bytes = capture(stdout).await?;
        if child
            .wait_for_clean_exit(deadline.into_std())
            .await
            .is_err()
        {
            return Err(anyhow::anyhow!(START_FAILED));
        }
        Ok::<_, anyhow::Error>((bytes, ()))
    })
    .await
    .context(TIMED_OUT)??;
    let connection = receipt::validate(profile, &bytes)?;
    let _verified = yolo_hub_client::Client::connect_local(
        &connection.socket_path,
        connection.entry_hub,
        deadline,
    )
    .await?;
    Ok(connection)
}

pub(crate) async fn capture(reader: impl AsyncRead + Unpin) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_READY_BYTES as u64 + OVERFLOW_BYTE)
        .read_to_end(&mut bytes)
        .await?;
    ensure!(bytes.len() <= MAX_READY_BYTES, OUTPUT_TOO_LARGE);
    Ok(bytes)
}
