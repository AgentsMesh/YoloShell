use crate::constants::AUDIT_LOCAL_ONLY;
use crate::selection::Selection;
use anyhow::{Result, bail};
use yolo_local_profile::LocalProfile;

pub(crate) async fn run(
    selection: &Selection,
    after: Option<u64>,
    limit: u16,
    json: bool,
    merge: &[std::path::PathBuf],
    trust: &[String],
    identity: bool,
    pull: Option<yolo_hub_identity::HubId>,
    output: Option<std::path::PathBuf>,
) -> Result<()> {
    let profile = profile(selection)?;
    yolo_tui_audit::run(
        &profile,
        yolo_tui_audit::Request {
            after,
            limit,
            json,
            merge,
            trust,
            identity,
            pull,
            output,
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
