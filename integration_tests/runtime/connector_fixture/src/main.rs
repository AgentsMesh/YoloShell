use clap::Parser;
use std::process::ExitCode;
use yolo_connector_fixture::{Arguments, run};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run(Arguments::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
