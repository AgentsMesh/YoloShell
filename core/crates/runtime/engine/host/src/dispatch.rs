use super::*;
use futures::StreamExt;

impl Host {
    pub(super) async fn dispatch(&self, request: &RequestEnvelope) -> ResponseEnvelope {
        if let Err(error) = self.verify_namespace() {
            return ResponseEnvelope::failure(request, internal(error));
        }
        let response = match self.dispatch_inner(request).await {
            Ok(response) => response,
            Err(error) => ResponseEnvelope::failure(request, error),
        };
        match self.verify_namespace() {
            Ok(()) => response,
            Err(error) => ResponseEnvelope::failure(request, internal(error)),
        }
    }

    pub(super) async fn dispatch_inner(
        &self,
        request: &RequestEnvelope,
    ) -> std::result::Result<ResponseEnvelope, ErrorBody> {
        match request.method.as_str() {
            method::HELLO => {
                let payload: HelloRequest = decode_request(request)?;
                if payload.protocol_version != crate::model::PROTOCOL_VERSION {
                    return Err(client_protocol_incompatible(payload.protocol_version));
                }
                respond(
                    request,
                    &HelloResult {
                        server_name: crate::protocol::HOST_SERVER_NAME.into(),
                        protocol_version: crate::model::PROTOCOL_VERSION,
                        capabilities: capabilities(),
                    },
                )
            }
            method::HOST_STATUS => respond(request, &self.host_status()),
            method::HOST_BUILD => respond(request, &build::current().map_err(internal)?),
            method::LIST_SESSIONS => {
                let payload: ListSessionsRequest = decode_request(request)?;
                let (manifests, complete) = if payload.require_complete {
                    (
                        self.catalog_list_complete(payload.include_archived)
                            .await
                            .map_err(internal)?,
                        true,
                    )
                } else {
                    let listing = self
                        .catalog_list_tolerant(payload.include_archived)
                        .await
                        .map_err(internal)?;
                    let complete = listing.diagnostics.is_empty();
                    for diagnostic in listing.diagnostics {
                        tracing::error!(
                            row_key = %diagnostic.row_key,
                            detail = %diagnostic.detail,
                            "catalog row omitted from session listing"
                        );
                    }
                    (listing.manifests, complete)
                };
                // A catalogue can contain many long-lived records. Resolve
                // their observations concurrently so one slow Worker cannot
                // consume the caller's whole RPC budget.
                let sessions = futures::stream::iter(manifests)
                    .map(|manifest| self.view_for_manifest(manifest))
                    .buffered(MAX_CATALOG_VIEW_CONCURRENCY)
                    .collect()
                    .await;
                respond(request, &ListSessionsResult { sessions, complete })
            }
            method::GET_SESSION => {
                let payload: TerminalRef = decode_request(request)?;
                let manifest = self
                    .catalog_get(payload.terminal_id)
                    .await
                    .map_err(internal)?
                    .ok_or_else(|| ErrorBody::not_found(SESSION_NOT_FOUND))?;
                respond(request, &self.get_session_view_for_manifest(manifest).await)
            }
            method::QUERY_CREATION => {
                let payload: CreateTerminal = decode_request(request)?;
                let result = self.query_creation(payload).await?;
                respond(request, &result)
            }
            method::WORKSPACE_INSPECT
            | method::WORKTREE_CREATE
            | method::WORKTREE_REMOVE
            | method::WORKTREE_LIST => self.workspace.dispatch(request),
            method::CREATE_SESSION => {
                let payload: CreateTerminal = decode_request(request)?;
                let result = self.create_session(payload).await?;
                respond(request, &result)
            }
            method::ARCHIVE_SESSION => {
                let payload: TerminalLifecycleSelection = decode_request(request)?;
                let view = self.archive_product_session(payload).await?;
                respond(request, &view)
            }
            method::TERMINATE_SESSION => {
                let payload: TerminalTerminateRequest = decode_request(request)?;
                self.terminate_product_request(payload).await?;
                Ok(ResponseEnvelope::empty(request))
            }
            method::DISCARD_SESSION => {
                let payload: TerminalDiscardRequest = decode_request(request)?;
                let view = self.discard_product_session(payload).await?;
                respond(request, &view)
            }
            method::CLEANUP_TERMINATE_SESSION => {
                let payload: CreationCleanupTerminateRequest = decode_request(request)?;
                let outcome = self.cleanup_terminate_session(payload).await?;
                respond(request, &outcome)
            }
            method::CLEANUP_ARCHIVE_SESSION => {
                let payload: CreationCleanupArchiveRequest = decode_request(request)?;
                let view = self.cleanup_archive_session(payload).await?;
                respond(request, &view)
            }
            method::CLEANUP_DISCARD_SESSION => {
                let payload: CreationCleanupDiscardRequest = decode_request(request)?;
                let view = self.cleanup_discard_session(payload).await?;
                respond(request, &view)
            }
            method::FINALIZE_CREATION => {
                let payload: FinalizeCreationRequest = decode_request(request)?;
                self.finalize_creation(payload).await?;
                Ok(ResponseEnvelope::empty(request))
            }
            method::RESUME_SESSION_EXACT => {
                let payload: ResumeRequest = decode_request(request)?;
                let view = self.resume_session(payload).await?;
                respond(request, &view)
            }
            method::READ_RETAINED_HISTORY => {
                let payload: yolo_runtime_protocol::RetainedHistoryRequest =
                    decode_request(request)?;
                let history = self.read_retained_history(payload).await?;
                respond(request, &history)
            }
            method::RECONCILE => respond(request, &self.reconcile().await),
            method::ACQUIRE_WRITER
            | method::RENEW_WRITER
            | method::RELEASE_WRITER
            | method::TAKEOVER_WRITER
            | method::GET_WRITER_LEASE
            | method::TERMINAL_INPUT
            | method::RESIZE_TERMINAL
            | method::SIGNAL_SESSION
            | method::OBSERVE_NATIVE_AGENT
            | method::GET_NATIVE_CONTROL_RECEIPT
            | method::INTERRUPT_NATIVE_AGENT
            | method::SUBMIT_NATIVE_AGENT
            | method::RESPOND_NATIVE_AGENT
            | method::GET_SNAPSHOT
            | method::GET_TEXT => self.proxy_runtime_bound_worker_request(request).await,
            _ => Err(ErrorBody::new(
                crate::protocol::error_code::UNSUPPORTED_METHOD,
                format!("unsupported method {}", request.method),
                false,
            )),
        }
    }

    pub(super) fn host_status(&self) -> HostStatus {
        HostStatus {
            protocol_version: crate::model::PROTOCOL_VERSION,
            boot_id: self.boot_id.clone(),
            host_pid: std::process::id(),
            controller_generation: self.controller_generation,
            home: self.config.home.clone(),
            capabilities: capabilities(),
        }
    }
}
