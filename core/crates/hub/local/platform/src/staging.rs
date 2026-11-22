use std::{fs, io::ErrorKind};
use yolo_client::Client;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{PublicError, StageAttachmentRequest, StagedAttachment};
use yolo_paths::{
    Paths, directory::PrivateDirectory, durable_mutation::SessionDurableMutationGuard,
    namespace_lease::NamespaceLease,
};
use yolo_terminal_model::{ExitRecord, RuntimeRecord, TerminalManifest, TerminalRuntimeStatus};

mod staging_file;
pub(super) const PENDING: &str = "pending";
pub(super) const FILES: &str = "files";

pub async fn stage_attachment(
    client: &Client,
    paths: &Paths,
    request: &StageAttachmentRequest,
) -> Result<Reply, PublicError> {
    let id = request.target.address.terminal_id.as_uuid();
    let view = client.get_session(id).await.map_err(crate::map_error)?;
    let runtime = view.runtime.ok_or(PublicError::NotFound)?;
    if runtime.runtime_epoch != request.target.runtime_epoch.as_uuid() {
        return Err(PublicError::TargetMismatch);
    }
    if view.manifest.archived
        || !matches!(
            runtime.status,
            TerminalRuntimeStatus::Running | TerminalRuntimeStatus::OutputBlocked
        )
    {
        return Err(PublicError::Conflict);
    }
    let paths = paths.clone();
    let request = request.clone();
    tokio::task::spawn_blocking(move || stage(&paths, &request))
        .await
        .map_err(|_| PublicError::Internal)?
        .map(|attachment| Reply::StagedAttachment { attachment })
}

fn stage(paths: &Paths, request: &StageAttachmentRequest) -> Result<StagedAttachment, PublicError> {
    let id = request.target.address.terminal_id.as_uuid();
    let epoch = request.target.runtime_epoch.as_uuid();
    let home =
        NamespaceLease::capture(&[("terminal attachment", paths.home())]).map_err(internal)?;
    // Serializes concurrent chunks, archive and runtime replacement across Hub processes.
    let guard = SessionDurableMutationGuard::acquire_participant(paths, &home, id)
        .map_err(|_| PublicError::Conflict)?;
    guard
        .require_no_pending_intent()
        .map_err(|_| PublicError::Conflict)?;
    let manifest = home
        .read_json::<TerminalManifest>(&paths.manifest(id))
        .map_err(internal)?;
    let runtime = home
        .read_json::<RuntimeRecord>(&paths.runtime_record(id))
        .map_err(internal)?;
    if runtime.terminal_id != id || runtime.runtime_epoch != epoch {
        return Err(PublicError::TargetMismatch);
    }
    if manifest.terminal_id != id || manifest.archived {
        return Err(PublicError::Conflict);
    }
    let exit = paths.exit_record(id);
    if !home.file_is_missing(&exit).map_err(internal)?
        && home.read_json::<ExitRecord>(&exit).map_err(internal)?.runtime_epoch == epoch
    {
        return Err(PublicError::Conflict);
    }
    let path = yolo_paths::attachments::directory(&paths.session_dir(id))
        .join(epoch.to_string())
        .join(request.upload_id.to_string());
    let upload = if request.offset == 0 {
        home.ensure_directory(&path).map_err(internal)?
    } else {
        home.directory(&path)
            .map_err(|_| PublicError::InvalidCursor)?
    };
    let path = store_chunk(&upload, request)?;
    guard.verify_session_directory().map_err(internal)?;
    Ok(StagedAttachment {
        target: request.target,
        upload_id: request.upload_id,
        next_offset: request.next_offset()?,
        path,
    })
}

pub(super) fn store_chunk(
    upload: &PrivateDirectory,
    request: &StageAttachmentRequest,
) -> Result<Option<String>, PublicError> {
    let published = upload.path().join(FILES);
    let pending = upload.path().join(PENDING);
    let complete = match fs::symlink_metadata(&published) {
        Ok(_) => true,
        Err(error) if error.kind() == ErrorKind::NotFound => false,
        Err(error) => return Err(internal(error)),
    };
    let dir = if complete {
        upload.open_descendant(&published).map_err(internal)?
    } else {
        upload.ensure_descendant(&pending).map_err(internal)?
    };
    // One basename per upload identity. The filename itself is durable metadata.
    let mut entries = fs::read_dir(dir.path()).map_err(internal)?;
    if let Some(entry) = entries.next().transpose().map_err(internal)?
        && (entry.file_name() != request.name.as_str() || entries.next().is_some())
    {
        return Err(PublicError::Conflict);
    }
    staging_file::apply(&dir, request, complete)?;
    if !request.eof {
        return Ok(None);
    }
    let file = published.join(&request.name);
    let path = file
        .to_str()
        .ok_or(PublicError::InvalidDirectory)?
        .to_owned();
    if path.chars().any(char::is_control) {
        return Err(PublicError::InvalidDirectory);
    }
    if !complete {
        dir.verify_path().map_err(internal)?;
        yolo_paths::private_file::rename_exclusive_between(
            upload.descriptor(),
            c"pending",
            upload.descriptor(),
            c"files",
        )
        .map_err(internal)?;
    }
    upload.sync().map_err(internal)?;
    upload.verify_path().map_err(internal)?;
    Ok(Some(path))
}

fn internal<E: std::fmt::Display>(_error: E) -> PublicError {
    PublicError::Internal
}
