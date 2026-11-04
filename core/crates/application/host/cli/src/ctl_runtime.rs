use anyhow::{Context, Result};
use yolo_client::Client;
use yolo_runtime::worker::RuntimeRef;

const RUNTIME_REQUIRED: &str = "session has no current runtime";
const AUTHORIZATION_SESSION_MISMATCH: &str =
    "writer authorization targets a different logical session";

pub(super) async fn runtime_target(client: &Client, terminal_id: uuid::Uuid) -> Result<RuntimeRef> {
    let runtime = client
        .get_session(terminal_id)
        .await?
        .runtime
        .context(RUNTIME_REQUIRED)?;
    Ok(RuntimeRef::new(terminal_id, runtime.runtime_epoch))
}

pub(crate) fn validate_mutation_authorization(
    terminal_id: uuid::Uuid,
    authorization: crate::ctl_args::RuntimeWriterAuthorization,
) -> Result<crate::ctl_args::RuntimeWriterAuthorization> {
    anyhow::ensure!(
        authorization.target.terminal_id == terminal_id,
        AUTHORIZATION_SESSION_MISMATCH
    );
    Ok(authorization)
}
