use crate::response_target::reply_matches_target;
use crate::{
    HUB_PROTOCOL_VERSION, MAX_CATALOG_PAGE_AGENTS, MAX_CATALOG_PAGE_HUBS,
    MAX_CATALOG_PAGE_TERMINALS, Method, OPERATIONS, Reply, Request,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hello {
    pub hub_id: HubId,
    pub version: u16,
    pub methods: Vec<Method>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub version: u16,
    #[serde(with = "crate::id")]
    pub request_id: Uuid,
    pub origin: HubId,
    pub method: Method,
    pub reply: Reply,
}

impl Response {
    pub fn new(request: &Request, origin: HubId, reply: Reply) -> Self {
        Self {
            version: HUB_PROTOCOL_VERSION,
            request_id: request.fields().request_id,
            origin,
            method: request.fields().command.method(),
            reply,
        }
    }

    pub fn validate_for(&self, request: &Request) -> Result<(), PublicError> {
        if self.version != HUB_PROTOCOL_VERSION {
            return Err(PublicError::UnsupportedSchema);
        }
        if self.origin != request.fields().destination {
            return Err(PublicError::OriginMismatch);
        }
        if self.request_id != request.fields().request_id
            || self.method != request.fields().command.method()
        {
            return Err(PublicError::InvalidRequest);
        }
        let valid = crate::reply_valid::valid(&self.reply, self.method, self.origin, self.version);
        if valid && reply_matches_target(&self.reply, &request.fields().command) {
            Ok(())
        } else {
            Err(PublicError::InvalidPayload)
        }
    }
}
