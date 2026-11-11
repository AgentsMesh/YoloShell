#[path = "bootstrap_contract.rs"]
mod contract;
#[path = "bootstrap_dispatch.rs"]
mod dispatch;
use contract::*;
use std::{
    io::{self, Write},
    path::PathBuf,
};
use yolo_launch_contract::HUB_STATE_ARGUMENT;
use yolo_process::ProcessIdentityObservation;

fn main() -> std::process::ExitCode {
    match dispatch::dispatch(std::env::args_os(), yolo_tui_cli::run_from, run_hub) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if let Some(error) = error.downcast_ref::<clap::Error>() {
                let _ = error.print();
                return std::process::ExitCode::from(error.exit_code() as u8);
            }
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run_hub(arguments: Vec<std::ffi::OsString>) -> anyhow::Result<()> {
    let parent_pid = u32::try_from(unsafe { libc::getppid() }).unwrap();
    assert!(parent_pid > INIT_PROCESS_ID);
    let parent = yolo_process::ProcessIdentity::capture(parent_pid).unwrap();
    let state = arguments
        .windows(ARGUMENT_PAIR_FIELDS)
        .find(|pair| pair[0] == HUB_STATE_ARGUMENT)
        .map(|pair| PathBuf::from(&pair[1]))
        .unwrap();
    let mode = state.file_name().unwrap().to_str().unwrap();
    if mode == FAILED {
        std::process::exit(EXIT_FAILURE);
    }
    if mode == OVERFLOW {
        publish_owned_helper(&state);
        while !state.join(RELEASE_FILE).exists() {
            if parent.observe() == ProcessIdentityObservation::Gone {
                return Ok(());
            }
            std::thread::park_timeout(POLL);
        }
        io::stdout()
            .write_all(&vec![
                b'x';
                yolo_hub_local_startup::MAX_READY_BYTES + OVERFLOW_BYTE
            ])
            .unwrap();
        io::stdout().flush().unwrap();
        // Oversized output aborts bootstrap. Keep this owned helper alive until
        // that cancellation retires it, rather than racing a normal LLVM flush.
        wait_for_owner_exit(parent);
        return Ok(());
    }
    if mode == MALFORMED {
        io::stdout().write_all(b"{}").unwrap();
        return Ok(());
    }
    assert_eq!(mode, HOLD);
    io::stdout().write_all(b"{").unwrap();
    io::stdout().flush().unwrap();
    publish_owned_helper(&state);
    wait_for_owner_exit(parent);
    Ok(())
}

fn publish_owned_helper(state: &std::path::Path) {
    std::fs::create_dir_all(state).unwrap();
    std::fs::write(state.join(PID_PENDING_FILE), std::process::id().to_string()).unwrap();
    std::fs::rename(state.join(PID_PENDING_FILE), state.join(PID_FILE)).unwrap();
}

fn wait_for_owner_exit(parent: yolo_process::ProcessIdentity) {
    while parent.observe() != ProcessIdentityObservation::Gone {
        std::thread::park_timeout(POLL);
    }
}
