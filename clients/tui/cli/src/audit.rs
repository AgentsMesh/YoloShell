use crate::constants::AUDIT_LOCAL_ONLY;
use crate::selection::Selection;
use anyhow::{Result, bail};
use yolo_local_profile::LocalProfile;

#[derive(Debug, clap::Args)]
pub(crate) struct AuditArgs {
    /// Only records after this sequence.
    #[arg(long)]
    pub(crate) after: Option<u64>,
    /// Maximum records to print.
    #[arg(long, default_value_t = crate::constants::DEFAULT_AUDIT_LIMIT)]
    pub(crate) limit: u16,
    /// Merge one or more audit JSONL files instead of reading the profile.
    #[arg(long = "merge", value_name = "FILE")]
    pub(crate) merge: Vec<std::path::PathBuf>,
    /// Trusted Hub identity for an authenticated merge: HUB_ID=PUBLIC_KEY.
    #[arg(long, value_name = "HUB_ID=PUBLIC_KEY")]
    pub(crate) trust: Vec<String>,
    /// Print this profile's Hub signing identity and exit.
    #[arg(long)]
    pub(crate) identity: bool,
    /// Pull signed audit lines from this peer through the local entry Hub.
    #[arg(long, value_name = "HUB_ID")]
    pub(crate) pull: Option<yolo_hub_identity::HubId>,
    /// Write pulled lines to this file instead of stdout.
    #[arg(long, value_name = "FILE", requires = "pull")]
    pub(crate) output: Option<std::path::PathBuf>,
    /// Print one JSON document.
    #[arg(long)]
    pub(crate) json: bool,
}

pub(crate) async fn run(selection: &Selection, args: AuditArgs) -> Result<()> {
    let profile = profile(selection)?;
    yolo_tui_audit::run(
        &profile,
        yolo_tui_audit::Request {
            after: args.after,
            limit: args.limit,
            json: args.json,
            merge: &args.merge,
            trust: &args.trust,
            identity: args.identity,
            pull: args.pull,
            output: args.output,
        },
    )
    .await
}

fn profile(selection: &Selection) -> Result<LocalProfile> {
    match selection {
        Selection::Local { profile, .. } => match profile.as_deref() {
            Some(path) => Ok(LocalProfile::load(path)?),
            None => Ok(yolo_local_profile::current()?),
        },
        Selection::Explicit(_) => bail!(AUDIT_LOCAL_ONLY),
    }
}
