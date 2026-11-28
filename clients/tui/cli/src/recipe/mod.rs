//! `yolo recipe`: a bounded JSON document that starts a sequence of agents in
//! one workspace. Starts are sequential and have no rollback.

mod document;
mod metadata;
mod order;
mod prompt;
mod run_query;
mod start;
mod wait;

pub(crate) use document::{
    RecipeAgent, RecipeDocument, RecipePrompt, RecipeWait, RecipeWaitUntil, read,
};
pub(crate) use metadata::merged;
pub(crate) use order::compute;
pub(crate) use prompt::RecipePromptWait;
pub(crate) use start::start;

use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub(crate) enum RecipeCommand {
    /// Start every agent in the recipe file, in order.
    #[command(group(crate::arguments::connection_group()))]
    Start(StartArgs),
    /// Validate and print a recipe file without starting anything.
    #[command(group(crate::arguments::connection_group()))]
    Show(ShowArgs),
}

#[derive(Debug, Args)]
pub(crate) struct StartArgs {
    /// Recipe JSON file to start.
    pub file: PathBuf,
    /// Stop already-started agents if a later start fails.
    #[arg(long)]
    pub stop_on_failure: bool,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub(crate) struct ShowArgs {
    /// Recipe JSON file to validate and print.
    pub file: PathBuf,
    /// Print one JSON document.
    #[arg(long)]
    pub json: bool,
}

pub(crate) fn show(args: &ShowArgs) -> Result<()> {
    let document = read(&args.file)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&document)?);
        return Ok(());
    }
    println!(
        "workspace: {}",
        document
            .workspace
            .map(|id| id.to_string())
            .unwrap_or_else(|| "-".into())
    );
    println!(
        "cwd: {}",
        document.cwd.as_deref().unwrap_or("workspace root")
    );
    if let Some(parent) = document.parent.as_deref() {
        println!("parent: {parent}");
    }
    if document.stop_on_failure {
        println!("stop_on_failure: true");
    }
    if !document.labels.is_empty() {
        println!("labels: {}", document.labels.len());
    }
    for agent in &document.agents {
        let after = if agent.after.is_empty() {
            String::new()
        } else {
            format!("\tafter={}", agent.after.join(","))
        };
        let wait = agent
            .wait
            .map(|wait| format!("\twait={}/{}s", wait.until.label(), wait.timeout_seconds))
            .unwrap_or_default();
        let prompt = agent
            .prompt
            .as_ref()
            .map(|prompt| format!("\tprompt={}", prompt.wait.label()))
            .unwrap_or_default();
        println!(
            "{}\t{}{after}{wait}{prompt}",
            agent.provider,
            agent.name.as_deref().unwrap_or("-")
        );
    }
    Ok(())
}
