use constants::*;
use std::{path::Path, process::ExitCode};
use tokio::{io::AsyncWriteExt, time::Instant};
use yolo_hub_file_executor_test_support::fault_lifecycle::rejects_while_alive;
use yolo_hub_file_executor_test_support::{constants, proof};
use yolo_hub_file_helper_protocol::{Command, MAX_OPERATION_TIME, Payload, Request, Response};
use yolo_hub_terminal_io::Descriptor;
use yolo_hub_transport::{Framed, Incoming};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => ExitCode::from(code),
        Err(_) => ExitCode::FAILURE,
    }
}

async fn run() -> anyhow::Result<u8> {
    let mut arguments = std::env::args_os();
    arguments.next();
    anyhow::ensure!(
        arguments.eq(yolo_launch_contract::ProcessRole::FileHelper
            .arguments()
            .map(std::ffi::OsString::from)),
        INVALID_LAUNCH_ARGUMENTS
    );
    yolo_hub_file_helper::watch_owner()?;
    let executable = std::env::current_exe()?;
    let mode = executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap();
    let deadline = Instant::now() + MAX_OPERATION_TIME;
    // Retain the original stdin endpoint through normal process exit. EMPTY
    // and TRUNCATED must publish EOF only after LLVM finishes its normal flush;
    // CLOSED_CONTROL explicitly models an earlier channel loss below.
    let mut channel = Framed::new(Descriptor::duplicate(libc::STDIN_FILENO)?);
    let Incoming::Message(request) = channel.receive::<Request>(deadline).await? else {
        return Ok(1);
    };
    if mode == LAUNCHER {
        let program = executable.parent().unwrap().join(HANG);
        let executor = yolo_hub_file_executor::FileExecutor::new(program)?;
        let _ = executor.execute(request, deadline).await;
        return Ok(0);
    }
    let directory = match &request.fields().command {
        Command::Build {} => executable.parent().unwrap().to_str().unwrap(),
        Command::Inspect { roots } => &roots[0].fields().path,
        Command::Read { binding, .. } => &binding.root.fields().path,
    };
    proof::Proof::capture()?.write(Path::new(directory))?;
    if mode == GATED_READ && matches!(request.fields().command, Command::Read { .. }) {
        std::fs::write(
            Path::new(directory).join(READ_READY_FILE),
            serde_json::to_vec(&proof::Proof::capture()?)?,
        )?;
        while !Path::new(directory).join(READ_RELEASE_FILE).is_file() {
            std::thread::sleep(POLL_INTERVAL);
        }
    }
    if mode == HANG {
        await_forced_exit();
    }
    if mode == EMPTY {
        return Ok(0);
    }
    if mode == CLOSED_CONTROL {
        drop(channel);
        unsafe {
            libc::close(libc::STDIN_FILENO);
        }
        await_forced_exit();
    }
    if mode == OVERSIZED {
        let mut stream = channel.into_stream()?;
        stream
            .write_all(&((yolo_hub_transport::MAX_FRAME_BYTES + 1) as u32).to_be_bytes())
            .await?;
        await_forced_exit();
    }
    let mut response = Response::new(&request, yolo_hub_file_helper::execute(&request));
    match mode {
        BAD_ID => response.request_id = uuid::Uuid::new_v4(),
        BAD_ORIGIN => response.origin = yolo_hub_identity::HubId::new(),
        BAD_SCHEMA => response.schema_version = u16::MAX,
        OMIT_ROOT => response.payload = Payload::Inspected { bindings: vec![] },
        _ => {}
    }
    if mode == TRUNCATED {
        let mut bytes = yolo_hub_transport::encode(&response)?;
        bytes.pop();
        channel.into_stream()?.write_all(&bytes).await?;
    } else {
        channel.send(&response, deadline).await?;
        if mode == DUPLICATE {
            channel.send(&response, deadline).await?;
        }
    }
    if rejects_while_alive(mode) {
        // Rejection owns retirement. Do not race a normal LLVM profile flush
        // against the executor's cancellation or owner-watchdog termination.
        await_forced_exit();
    }
    Ok(if mode == FAIL_EXIT { 1 } else { 0 })
}

fn await_forced_exit() -> ! {
    loop {
        std::thread::park();
    }
}
