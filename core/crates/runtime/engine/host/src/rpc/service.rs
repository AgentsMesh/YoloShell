use super::super::*;
use yolo_capnp_rpc::{Method, Protocol, Reply, Service, Value};

pub(in crate::host) async fn serve(host: Arc<Host>, stream: UnixStream) -> Result<()> {
    verify_peer_uid(&stream)?;
    let (activity, observer) = tokio::sync::watch::channel(0);
    let service = Arc::new(super::activity::ActivityService { host, activity });
    tokio::select! {
        result = yolo_capnp_rpc::serve(stream, Protocol::Endpoint, service) => result.map_err(Into::into),
        result = super::activity::idle(observer) => result,
    }
}

impl Service for Host {
    fn call(&self, method_kind: Method, value: Value) -> yolo_capnp_rpc::Future<'_, Reply> {
        Box::pin(async move {
            if method_kind != Method::Call {
                return Err(yolo_capnp_rpc::Error::Invalid);
            }
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            self.verify_namespace().map_err(rpc_error)?;
            let publication = self
                .request_drain
                .admit_response()
                .map_err(|error| rpc_error(error.message))?;
            if request.version != crate::model::PROTOCOL_VERSION
                || request.kind != crate::protocol::REQUEST_KIND
            {
                return reply(
                    ResponseEnvelope::failure(&request, request_protocol_incompatible()),
                    publication,
                    None,
                );
            }
            if request.method == method::ATTACH_SESSION {
                return match self.rpc_attachment(&request).await {
                    Ok((response, events, lock)) => Ok(Reply {
                        value: yolo_capnp_codec::to_value(&response)?,
                        events: Some(events),
                        on_observed: Some(Box::new(move || {
                            drop(lock);
                            publication.publish();
                        })),
                    }),
                    Err(error) => reply(
                        ResponseEnvelope::failure(&request, error),
                        publication,
                        None,
                    ),
                };
            }
            if request.method == method::WATCH_CATALOGUE {
                let (revision, source) = self.watch.feed.subscribe();
                let start = yolo_runtime_protocol::WatchCatalogueStart { revision };
                let value = ResponseEnvelope::success(&request, &start)
                    .map_err(|error| yolo_capnp_rpc::Error::Remote(error.to_string()))?;
                return Ok(Reply {
                    value: yolo_capnp_codec::to_value(&value)?,
                    events: Some(Box::new(source)),
                    on_observed: Some(Box::new(move || publication.publish())),
                });
            }
            if request.method == method::SHUTDOWN_IF_IDLE {
                return match self.prepare_idle_shutdown().await {
                    Ok(preparation) => Ok(Reply {
                        value: yolo_capnp_codec::to_value(&ResponseEnvelope::empty(&request))?,
                        events: None,
                        on_observed: Some(Box::new(move || {
                            publication.publish();
                            preparation.commit();
                        })),
                    }),
                    Err(error) => reply(
                        ResponseEnvelope::failure(&request, error),
                        publication,
                        None,
                    ),
                };
            }
            let mutation = if yolo_host_change_feed::mutates(&request.method) {
                match self.request_drain.admit_mutation().await {
                    Ok(value) => Some(value),
                    Err(error) => {
                        return reply(
                            ResponseEnvelope::failure(&request, error),
                            publication,
                            None,
                        );
                    }
                }
            } else {
                None
            };
            let response = self.dispatch(&request).await;
            self.verify_namespace().map_err(rpc_error)?;
            if response.ok && yolo_host_change_feed::changes_catalogue(&request.method) {
                self.watch.feed.touch(None);
            }
            reply(response, publication, mutation)
        })
    }
}

fn reply(
    response: ResponseEnvelope,
    publication: request_drain::HostResponseTicket,
    mutation: Option<request_drain::HostMutationPermit>,
) -> std::result::Result<Reply, yolo_capnp_rpc::Error> {
    Ok(Reply {
        value: yolo_capnp_codec::to_value(&response)?,
        events: None,
        on_observed: Some(Box::new(move || {
            publication.publish();
            drop(mutation);
        })),
    })
}

fn rpc_error(error: impl std::fmt::Display) -> yolo_capnp_rpc::Error {
    yolo_capnp_rpc::Error::Remote(error.to_string())
}
