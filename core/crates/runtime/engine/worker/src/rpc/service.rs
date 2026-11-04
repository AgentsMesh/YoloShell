use super::super::*;
use yolo_capnp_rpc::{EventSource, Method, Protocol, Reply, Service, Value};
use yolo_runtime_protocol::WatchAgentEventsRequest;

pub(in crate::worker) async fn serve(
    stream: UnixStream,
    state: Arc<WorkerState>,
    controller: AuthenticatedController,
) -> Result<()> {
    yolo_capnp_rpc::serve(
        stream,
        Protocol::Endpoint,
        Arc::new(WorkerService { state, controller }),
    )
    .await?;
    Ok(())
}

struct WorkerService {
    state: Arc<WorkerState>,
    controller: AuthenticatedController,
}

async fn subscribe_attachment_events(state: &WorkerState) -> broadcast::Receiver<WorkerEvent> {
    let receiver = state.events.subscribe();
    #[cfg(test)]
    state.control_test_hooks.attachment_after_subscribe.pause_if_armed().await;
    receiver
}
impl Service for WorkerService {
    fn call(&self, method: Method, value: Value) -> yolo_capnp_rpc::Future<'_, Reply> {
        Box::pin(async move {
            if method != Method::Call {
                return Err(yolo_capnp_rpc::Error::Invalid);
            }
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            #[cfg(test)]
            self.state
                .control_test_hooks
                .response_before_admission
                .pause_if_armed()
                .await;
            let publication = match self.state.reserve_response_publication() {
                Ok(value) => value,
                Err(detail) => {
                    return response(
                        ResponseEnvelope::failure(
                            &request,
                            ErrorBody::worker_shutting_down(detail),
                        ),
                        None,
                        None,
                    );
                }
            };
            let outcome = if request.version != crate::model::PROTOCOL_VERSION
                || request.kind != crate::protocol::REQUEST_KIND
            {
                Err(ErrorBody::protocol_incompatible(super::INVALID_RPC))
            } else if let Err(error) = self.state.validate_controller(self.controller) {
                Err(error)
            } else if request.method == crate::protocol::method::ATTACH_SESSION {
                self.attach(&request).await
            } else if request.method == crate::protocol::method::WATCH_AGENT_EVENTS {
                self.watch_agent_events(&request)
            } else {
                Ok((
                    dispatch_request(&request, &self.state, self.controller).await,
                    None,
                ))
            };
            let (envelope, events) =
                outcome.unwrap_or_else(|error| (ResponseEnvelope::failure(&request, error), None));
            #[cfg(test)]
            self.state
                .control_test_hooks
                .response_before_write
                .pause_if_armed()
                .await;
            response(
                envelope,
                events,
                Some(Box::new(move || publication.publish())),
            )
        })
    }
}

impl WorkerService {
    fn watch_agent_events(
        &self,
        request: &RequestEnvelope,
    ) -> Result<(ResponseEnvelope, Option<Box<dyn EventSource>>), ErrorBody> {
        let payload: WatchAgentEventsRequest = decode_worker_payload(request)?;
        self.state.validate_runtime(payload.terminal_id, payload.runtime_epoch)?;
        let source = yolo_worker_agent_events::AgentEventSource::new(
            payload.terminal_id, payload.runtime_epoch, self.state.agent_events.subscribe());
        Ok((ResponseEnvelope::empty(request), Some(Box::new(source))))
    }

    async fn attach(
        &self,
        request: &RequestEnvelope,
    ) -> std::result::Result<(ResponseEnvelope, Option<Box<dyn EventSource>>), ErrorBody> {
        let capacity = crate::ingress::try_admit(&self.state.attachment_clients)
            .ok_or_else(|| ErrorBody::conflict(WORKER_ATTACHMENT_LIMIT_MESSAGE))?;
        let payload: AttachRequest = decode_worker_payload(request)?;
        self.state.validate_session(payload.terminal_id)?;
        self.state.validate_attachment_cursor(payload.after_seq)?;
        let receiver = subscribe_attachment_events(&self.state).await;
        let permit = self
            .state
            .admit_controller_mutation(self.controller)
            .await?;
        self.state.ensure_snapshot_or_attachment_allowed()?;
        let attached = self
            .state
            .attach_result_admitted(&permit, payload.after_seq)
            .map_err(|error| error.into_error_body())?;
        let barrier = attachment_observed_sequence(attached.next_sequence)
            .ok_or_else(|| AttachmentResultError::CursorExhausted.into_error_body())?;
        let completed = lock_recover(&self.state.exit).clone();
        let response = ResponseEnvelope::success(request, &attached).map_err(protocol_error)?;
        let source =
            super::events::Events::new(self.state.clone(), receiver, barrier, capacity, completed);
        Ok((response, Some(Box::new(source))))
    }
}

fn response(
    envelope: ResponseEnvelope,
    events: Option<Box<dyn EventSource>>,
    on_observed: Option<Box<dyn yolo_capnp_rpc::Confirmation>>,
) -> std::result::Result<Reply, yolo_capnp_rpc::Error> {
    Ok(Reply {
        value: yolo_capnp_codec::to_value(&envelope)?,
        events,
        on_observed,
    })
}
