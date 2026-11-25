use clap::Subcommand;
use std::path::PathBuf;

/// User-facing provider hook management. The Host role keeps the
/// revision-fenced commands for automation; this surface owns the review,
/// confirmation and readable output.
#[derive(Debug, Subcommand)]
pub enum HookCommand {
    /// Show which terminal agents have managed hooks installed.
    Status {
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
    /// Install managed hooks for one or more terminal agents.
    Install {
        /// Provider tokens: claude, codex, qwen, gemini or grok.
        #[arg(required = true)]
        providers: Vec<String>,
        /// Provider configuration directory; only with exactly one provider.
        #[arg(long)]
        config_home: Option<PathBuf>,
        /// Install without asking for confirmation.
        #[arg(long)]
        yes: bool,
        /// Review the changes without writing any file.
        #[arg(long)]
        dry_run: bool,
        /// Print one JSON document.
        #[arg(long)]
        json: bool,
    },
}
