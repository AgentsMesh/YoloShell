use super::PtyRuntimeAliasReleasePoll;
use anyhow::Result;

pub(crate) const HOST_RELEASE_RETRY_DETAIL: &str =
    "retrying exact PTY custody release with successor Host authority";

pub(crate) fn after_error(error: anyhow::Error) -> Result<PtyRuntimeAliasReleasePoll> {
    let detail = super::super::terminal_reporting::error_text(error);
    after_detail(&detail)
}

pub(crate) fn after_detail(detail: &str) -> Result<PtyRuntimeAliasReleasePoll> {
    super::super::terminal_reporting::warn_detail(detail, HOST_RELEASE_RETRY_DETAIL);
    Ok(PtyRuntimeAliasReleasePoll::Pending)
}
