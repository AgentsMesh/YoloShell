use super::bootstrap_exec_observer::current_exec_thread_id;
use super::bootstrap_gate::{
    BootstrapCapabilityAck, BootstrapChildSpec, BootstrapExecFailureFrame, BootstrapGateReply,
};
use super::constants::*;
use crate::local_unix::LocalUnixStream;
use crate::protocol::{read_frame, write_frame};
use anyhow::{Context, Result, ensure};
#[cfg(not(target_os = "macos"))]
use std::os::unix::process::CommandExt;
use tokio::time::timeout;

pub(crate) const BOOTSTRAP_GATE_PREPARE_REPLY_CONTEXT: &str =
    "invalid bootstrap gate preparation reply";
pub(crate) const BOOTSTRAP_GATE_RELEASE_READ_CONTEXT: &str = "await bootstrap gate release";

pub async fn run_bootstrap_child() -> Result<()> {
    let encoded = std::env::var(BOOTSTRAP_GATE_SPEC_ENV).context(BOOTSTRAP_SPEC_MISSING_CONTEXT)?;
    let spec: BootstrapChildSpec =
        super::bootstrap_spec_codec::decode(&encoded).context(BOOTSTRAP_SPEC_DECODE_CONTEXT)?;
    let hello = spec.hello()?;
    let mut stream = timeout(
        BOOTSTRAP_GATE_TIMEOUT,
        LocalUnixStream::connect(&spec.socket_path),
    )
    .await
    .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
    super::local_transport::verify_same_uid(&stream)?;
    timeout(BOOTSTRAP_GATE_TIMEOUT, write_frame(&mut stream, &hello))
        .await
        .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
    #[cfg(target_os = "macos")]
    // Keep descriptor custody through exec when the target permits it. A target that
    // closes unknown descriptors is handled only after exec and inventory are fenced.
    let mut process_capability = std::mem::ManuallyDrop::new(
        super::bootstrap_gate::descriptor::receive_before(
            &stream,
            tokio::time::Instant::now() + BOOTSTRAP_GATE_TIMEOUT,
        )
        .await?,
    );
    #[cfg(target_os = "macos")]
    {
        let identity = process_capability.identity();
        let acknowledgement =
            capability_acknowledgement(identity.reader_handle(), identity.writer_handle());
        timeout(
            BOOTSTRAP_GATE_TIMEOUT,
            write_frame(&mut stream, &acknowledgement),
        )
        .await
        .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
    }
    let prepared: BootstrapGateReply = timeout(BOOTSTRAP_GATE_TIMEOUT, read_frame(&mut stream))
        .await
        .context(BOOTSTRAP_GATE_TIMEOUT_CONTEXT)??;
    ensure!(
        matches!(prepared, BootstrapGateReply::Prepared),
        BOOTSTRAP_GATE_PREPARE_REPLY_CONTEXT
    );
    await_release(&mut stream).await?;
    #[cfg(test)]
    super::bootstrap_exec_test_hook::pause_before_exec()?;
    #[cfg(test)]
    if super::bootstrap_exec_test_hook::inject_exec_report(&mut stream).await? {
        anyhow::bail!(super::bootstrap_exec_test_hook::EXEC_REPORT_STOP_DETAIL);
    }
    let error = execute(
        &spec,
        hello.exec_thread_id,
        #[cfg(target_os = "macos")]
        &mut process_capability,
    );
    let failure = BootstrapExecFailureFrame {
        detail: error.to_string(),
    };
    timeout(BOOTSTRAP_GATE_TIMEOUT, write_frame(&mut stream, &failure))
        .await
        .context(BOOTSTRAP_EXEC_REPORT_CONTEXT)??;
    Err(error).context(BOOTSTRAP_EXEC_FAILURE_CONTEXT)
}

#[cfg(not(test))]
pub(crate) fn capability_acknowledgement(
    reader_handle: u64,
    writer_handle: u64,
) -> BootstrapCapabilityAck {
    BootstrapCapabilityAck {
        reader_handle,
        writer_handle,
    }
}

#[cfg(test)]
pub(crate) fn capability_acknowledgement(
    reader_handle: u64,
    writer_handle: u64,
) -> BootstrapCapabilityAck {
    if std::env::var_os(BOOTSTRAP_TEST_BAD_ACK_ENV).is_some() {
        return BootstrapCapabilityAck {
            reader_handle: writer_handle,
            writer_handle: reader_handle,
        };
    }
    BootstrapCapabilityAck {
        reader_handle,
        writer_handle,
    }
}

pub(crate) fn execute(
    spec: &BootstrapChildSpec,
    expected_thread: u32,
    #[cfg(target_os = "macos")]
    process_capability: &mut crate::process::MacosProcessCapabilityChildWriter,
) -> anyhow::Error {
    if let Err(error) = validate_exec_thread(expected_thread) {
        return error;
    }
    let mut command = std::process::Command::new(&spec.command);
    command.args(&spec.args).env_remove(BOOTSTRAP_GATE_SPEC_ENV);
    #[cfg(target_os = "macos")]
    return process_capability.exec_inheriting(&mut command);
    #[cfg(not(target_os = "macos"))]
    command.exec().into()
}

pub(crate) fn validate_exec_thread(expected: u32) -> Result<()> {
    ensure!(
        current_exec_thread_id()? == expected,
        BOOTSTRAP_EXEC_THREAD_CHANGED_CONTEXT
    );
    Ok(())
}

pub(crate) async fn await_release(stream: &mut LocalUnixStream) -> Result<()> {
    let reply: BootstrapGateReply = read_frame(stream)
        .await
        .context(BOOTSTRAP_GATE_RELEASE_READ_CONTEXT)?;
    ensure!(
        matches!(reply, BootstrapGateReply::Go),
        BOOTSTRAP_GATE_REPLY_CONTEXT
    );
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-bootstrap"))]
#[path = "../../unit_tests/bootstrap/bootstrap_child_wait_tests.rs"]
mod wait_tests;
