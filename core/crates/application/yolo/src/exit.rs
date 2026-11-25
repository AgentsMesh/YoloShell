use std::{fmt::Display, process::ExitCode};
use yolo_launch_contract::ProcessRole;

const USAGE_ERROR_CODE: u8 = 2;

pub fn usage(error: impl Display) -> ExitCode {
    eprintln!("{error}");
    ExitCode::from(USAGE_ERROR_CODE)
}

pub fn finish(role: Option<ProcessRole>, result: anyhow::Result<()>) -> ExitCode {
    let Err(error) = result else {
        return ExitCode::SUCCESS;
    };
    if let Some(error) = error.downcast_ref::<clap::Error>() {
        if error.print().is_err() {
            return ExitCode::FAILURE;
        }
        return u8::try_from(error.exit_code())
            .map(ExitCode::from)
            .unwrap_or(ExitCode::FAILURE);
    }
    if role != Some(ProcessRole::FileHelper) {
        eprintln!("{error:#}");
    }
    ExitCode::FAILURE
}
