use super::ctl_args::{
    CleanupAuthority, CreateAuthority, CreationAuthorization, FinalizeAuthority,
};
use std::path::PathBuf;
use yolo_runtime::model::{CleanupTerminationOutcome, CreateTerminal, DEFAULT_COLS, DEFAULT_ROWS};

async fn create_session(
    client: &Client,
    authorization: CreationAuthorization,
    export: bool,
    name: Option<String>,
    cwd: PathBuf,
) -> Result<()> {
    validate_creation_export(export, std::io::stdout().is_terminal())?;
    let authority = authorization.read::<CreateAuthority>()?.validate()?;
    let result = client
        .create_session(&CreateTerminal {
            creation_token: authority.creation_token,
            name,
            cwd,
            cols: Some(DEFAULT_COLS),
            rows: Some(DEFAULT_ROWS),
            initial_input: None,
            context: None,
        })
        .await?;
    print_creation_result(&result, export)
}

async fn cleanup_terminate(
    client: &Client,
    authorization: CreationAuthorization,
    terminal_id: uuid::Uuid,
    force: bool,
) -> Result<CleanupTerminationOutcome> {
    let authority = authorization.read::<CleanupAuthority>()?.validate()?;
    Ok(client
        .cleanup_terminate(terminal_id, authority.creation_capability, force)
        .await?)
}

async fn cleanup_archive(
    client: &Client,
    authorization: CreationAuthorization,
    terminal_id: uuid::Uuid,
) -> Result<yolo_runtime::model::TerminalView> {
    let authority = authorization.read::<CleanupAuthority>()?.validate()?;
    Ok(client
        .cleanup_archive(terminal_id, authority.creation_capability)
        .await?)
}

async fn cleanup_discard(
    client: &Client,
    authorization: CreationAuthorization,
    terminal_id: uuid::Uuid,
    force: bool,
) -> Result<yolo_runtime::model::TerminalView> {
    let authority = authorization.read::<CleanupAuthority>()?.validate()?;
    Ok(client
        .cleanup_discard(terminal_id, authority.creation_capability, force)
        .await?)
}

async fn finalize_creation(
    client: &Client,
    authorization: CreationAuthorization,
    terminal_id: uuid::Uuid,
) -> Result<()> {
    let authority = authorization.read::<FinalizeAuthority>()?.validate()?;
    Ok(client
        .finalize_creation(
            authority.creation_token,
            terminal_id,
            authority.creation_capability,
        )
        .await?)
}
