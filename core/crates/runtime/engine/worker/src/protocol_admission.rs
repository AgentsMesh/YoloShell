use super::{
    ErrorBody, RequestEnvelope, ResponseEnvelope, UnixStream, WorkerState, response_drain,
};
use anyhow::Result;

const WORKER_REQUEST_PROTOCOL_INCOMPATIBLE: &str =
    "worker request envelope is incompatible; worker supports protocol version";

pub(super) async fn admit(
    stream: &mut UnixStream,
    state: &std::sync::Arc<WorkerState>,
    request: &RequestEnvelope,
) -> Result<Option<response_drain::ResponsePublicationTicket>> {
    let Some(publication) = response_drain::admit(stream, state, request).await? else {
        return Ok(None);
    };
    if request.version == crate::model::PROTOCOL_VERSION
        && request.kind == crate::protocol::REQUEST_KIND
    {
        return Ok(Some(publication));
    }
    let detail = format!(
        "{WORKER_REQUEST_PROTOCOL_INCOMPATIBLE} {}",
        crate::model::PROTOCOL_VERSION
    );
    let response = ResponseEnvelope::failure(request, ErrorBody::protocol_incompatible(detail));
    response_drain::write(stream, state, publication, &response).await?;
    Ok(None)
}
