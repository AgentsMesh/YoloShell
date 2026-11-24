use crate::pty_custody::{CustodyHolderIdentity, CustodyRequest};

const CUSTODY_REQUEST_DEBUG_NAME: &str = "CustodyRequest";

impl std::fmt::Debug for CustodyRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct(CUSTODY_REQUEST_DEBUG_NAME)
            .field(yolo_wire::field::PROTOCOL_VERSION, &self.protocol_version)
            .field(yolo_wire::field::LEASE_ID, &self.lease_id)
            .field(yolo_wire::field::RUNTIME, &self.runtime)
            .field(yolo_wire::field::HOLDER, &self.holder)
            .field(
                yolo_wire::field::CONTROLLER_GENERATION,
                &self.controller_generation,
            )
            .finish_non_exhaustive()
    }
}

impl CustodyRequest {
    pub fn holder(&self) -> CustodyHolderIdentity {
        self.holder
    }

    pub fn auth_token(&self) -> &str {
        &self.auth_token
    }
}
