use crate::{LocalBackend, errors::map_error, summary::summary};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{PublicError, TerminalAction, TerminalCommand};
use yolo_terminal_model::{TerminalLifecycleSelection, TerminalManifestDigest, TerminalRuntimeStatus, TerminalView};

impl LocalBackend {
    pub(crate) async fn lifecycle(&self, request: TerminalCommand) -> Result<Reply, PublicError> {
        request.validate()?;
        let address = request.selection.address;
        let view = self
            .client
            .get_session(address.terminal_id.as_uuid())
            .await
            .map_err(map_error)?;
        let selected = selection_from_view(self.hub, request, &view)?;
        let current = match request.action {
            TerminalAction::Resume {} => self
                .client
                .resume_with_preconditions(selected, false)
                .await
                .map_err(map_error)?,
            TerminalAction::Recreate {} => self
                .client
                .resume_with_preconditions(selected, true)
                .await
                .map_err(map_error)?,
            TerminalAction::Terminate { force } => {
                self.client
                    .terminate_with_preconditions(selected, force)
                    .await
                    .map_err(map_error)?;
                self.client
                    .get_session(address.terminal_id.as_uuid())
                    .await
                    .map_err(map_error)?
            }
            TerminalAction::Archive {} => self
                .client
                .archive_with_preconditions(selected)
                .await
                .map_err(map_error)?,
        };
        Ok(Reply::Terminal {
            terminal: yolo_hub_local_platform::annotate_workspace(
                self.platform.as_deref(),
                summary(self.hub, &current)?,
            )
            .await?,
        })
    }
}

pub(crate) fn selection_from_view(
    hub: HubId,
    request: TerminalCommand,
    view: &TerminalView,
) -> Result<TerminalLifecycleSelection, PublicError> {
    request.validate()?;
    request.selection.validate_against(&summary(hub, view)?)?;
    let mut selected = TerminalLifecycleSelection::selected(
        view.manifest.terminal_id,
        view.manifest.updated_at_ms,
        view.runtime
            .as_ref()
            .map(|runtime| runtime.status)
            .unwrap_or(TerminalRuntimeStatus::Unreachable),
    );
    selected.expected_manifest_sha256 =
        Some(TerminalManifestDigest::capture(&view.manifest).map_err(|_| PublicError::Internal)?);
    selected.expected_runtime_epoch = request
        .selection
        .expected_runtime
        .map(|epoch| epoch.as_uuid());
    selected.require_runtime_absent = request.selection.expected_runtime.is_none();
    Ok(selected)
}
