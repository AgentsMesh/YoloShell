use crate::{
    LocalBackend, connector_catalogue::ObservationScope, errors::map_error,
    observation::SweepPolicy, platform_catalogue::terminal_summary,
};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_rpc_protocol::{Command, MAX_CATALOG_PAGE_TERMINALS, Reply};
use yolo_hub_terminal_protocol::{PublicError, TerminalSummary};
use yolo_terminal_model::TerminalView;

impl LocalBackend {
    pub fn with_platform(
        mut self,
        workspaces: yolo_paths::directory::PrivateDirectory,
        runs: yolo_paths::directory::PrivateDirectory,
    ) -> Result<Self, PublicError> {
        self.platform = Some(yolo_hub_local_platform::PlatformServices::shared(
            workspaces,
            runs,
            self.hub,
            self.client.clone(),
        )?);
        Ok(self)
    }

    pub(crate) async fn platform_dispatch(
        &self,
        command: &Command,
    ) -> Option<Result<yolo_hub_rpc_protocol::Reply, PublicError>> {
        let platform = self.platform.as_ref()?;
        if matches!(
            command,
            Command::GetAgentRun { .. } | Command::ListAgentRuns { .. }
        ) && let Err(error) = self.refresh_runs().await
        {
            return Some(Err(error));
        }
        let mut reply = platform.dispatch(command).await?;
        if let Ok(reply @ (Reply::AgentRun { .. } | Reply::AgentRuns { .. })) = &mut reply {
            reply.annotate_run_current(&self.observed_targets().await);
        }
        Some(reply)
    }

    pub(crate) async fn catalogue_views(
        &self,
        include_archived: bool,
    ) -> Result<Vec<TerminalView>, yolo_client::ClientError> {
        self.catalogue.views(&self.client, include_archived).await
    }

    pub async fn watch_catalogue(
        &self,
        after_revision: u64,
    ) -> Result<yolo_client::CatalogueWatch, yolo_client::ClientError> {
        self.client.watch_catalogue(after_revision).await
    }
    pub async fn follow_catalogue_lifecycle(&self) -> std::sync::Arc<()> {
        self.catalogue.follow_lifecycle().await
    }

    pub async fn invalidate_catalogue(&self) {
        self.catalogue.invalidate().await;
    }

    pub async fn catalogue_changed(&self, ids: &std::collections::BTreeSet<uuid::Uuid>) {
        self.catalogue.changed(ids).await;
    }

    pub(crate) async fn list_page(
        &self,
        include_archived: bool,
        after: Option<TerminalAddress>,
    ) -> Result<Reply, PublicError> {
        let mut terminals = self.terminals(include_archived).await?;
        terminals.retain(|session| after.is_none_or(|after| session.fields().address > after));
        terminals.sort_by_key(|session| session.fields().address);
        let more = terminals.len() > MAX_CATALOG_PAGE_TERMINALS;
        terminals.truncate(MAX_CATALOG_PAGE_TERMINALS);
        let next = if more {
            terminals.last().map(|session| session.fields().address)
        } else {
            None
        };
        Ok(Reply::Terminals { terminals, next })
    }

    /// Gather Host and Worker evidence before taking the projection lock.
    pub async fn terminals(&self, archived: bool) -> Result<Vec<TerminalSummary>, PublicError> {
        let _sweep = self.begin_observation().await;
        let applied = self
            .apply_complete_catalogue(archived, SweepPolicy::Catalogue)
            .await?;
        self.terminal_summaries_from_applied(applied, archived)
            .await
    }

    pub async fn session(&self, address: TerminalAddress) -> Result<TerminalSummary, PublicError> {
        if address.hub_id != self.hub {
            return Err(PublicError::OriginMismatch);
        }
        let _sweep = self.begin_observation().await;
        let sweep = self
            .observe_catalogue(ObservationScope::Terminal(address), false)
            .await;
        let applied = self
            .apply_catalogue(sweep, false, SweepPolicy::Catalogue)
            .await?;
        let views = match applied.views {
            Ok(views) => views,
            Err(error) => return Err(map_error(error)),
        };
        let Some(view) = views.first() else {
            return Err(PublicError::Internal);
        };
        if view.manifest.terminal_id != address.terminal_id.as_uuid() {
            return Err(PublicError::TargetMismatch);
        }
        terminal_summary(
            self,
            view,
            applied.projected.first().and_then(Option::as_ref),
            &applied.connectors,
            &yolo_hub_local_platform::terminal_workspaces(self.platform.as_deref()).await?,
        )
    }
}

pub(crate) fn dimensions(
    backend: &LocalBackend,
    view: &TerminalView,
) -> Option<yolo_hub_terminal_protocol::TerminalSize> {
    let observed = view.runtime.as_ref()?;
    let record: yolo_terminal_model::RuntimeRecord =
        yolo_paths::read_json(&backend.paths.runtime_record(view.manifest.terminal_id)).ok()?;
    if record.terminal_id != view.manifest.terminal_id
        || record.runtime_epoch != observed.runtime_epoch
        || record.worker_id != observed.worker_id
        || record.worker_pid != observed.worker_pid
        || record.worker_start_time != observed.worker_start_time
        || record.child_pid != observed.child_pid
        || record.child_start_time != observed.child_start_time
    {
        return None;
    }
    yolo_hub_terminal_protocol::TerminalSize::new(record.cols, record.rows).ok()
}
