//! User-facing local service lifecycle commands.
mod render;

use anyhow::Result;
use clap::Args;
use render::{print_status, print_stop, print_version};
use yolo_local_lifecycle::{StatusReport, StopReport, restart, start, status, stop};
use yolo_local_profile::LocalProfile;

#[derive(Clone, Copy, Debug, Args)]
pub struct FormatOptions {
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

pub async fn status_command(profile: &LocalProfile, options: FormatOptions) -> Result<()> {
    print_status(&status(profile).await?, options.json)
}

pub async fn version_command(profile: &LocalProfile, options: FormatOptions) -> Result<()> {
    print_version(&status(profile).await?, options.json)
}

pub async fn start_command(profile: &LocalProfile, options: FormatOptions) -> Result<()> {
    start(profile, &std::env::current_exe()?).await?;
    print_status(&status(profile).await?, options.json)
}

pub async fn stop_command(profile: &LocalProfile, options: FormatOptions) -> Result<()> {
    print_stop(&stop(profile).await?, options.json)
}

pub async fn restart_command(profile: &LocalProfile, options: FormatOptions) -> Result<()> {
    let stopped = restart(profile, &std::env::current_exe()?).await?;
    let report = status(profile).await?;
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&RestartReport {
                stop: stopped,
                status: report,
            })?
        );
    } else {
        render::print_stop_lines(&stopped);
        render::status_lines(&report)
            .iter()
            .for_each(|line| println!("{line}"));
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct RestartReport {
    stop: StopReport,
    status: StatusReport,
}
