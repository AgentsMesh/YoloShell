use anyhow::{Context, Result};
use std::process::Child;
use yolo_process::ProcessIdentity;

pub(super) fn force_unreaped_direct_child(
    mut child: &mut Child,
    identity: Option<ProcessIdentity>,
    context: &'static str,
) -> Result<()> {
    let exact_failure = match identity.map(|identity| identity.signal(libc::SIGKILL)) {
        Some(Ok(true)) => return Ok(()),
        Some(Err(error)) => Some(error),
        Some(Ok(false)) | None => None,
    };
    let direct = ::std::process::Child::kill(&mut child).context(context);
    match (direct, exact_failure) {
        (Ok(()), _) => Ok(()),
        (Err(error), None) => Err(error),
        (Err(error), Some(exact)) => Err(error).context(format!("{context}: {exact:#}")),
    }
}
