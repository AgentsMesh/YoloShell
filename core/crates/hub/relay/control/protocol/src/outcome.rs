use crate::{Outcome, ProtocolError, Request};
impl Outcome {
    pub fn validate_for(self, request: &Request) -> Result<(), ProtocolError> {
        if let Self::Receipt { receipt } = self {
            let fields = request.fields().request.fields();
            if receipt.request_id() != fields.request_id || receipt.target() != fields.target {
                return Err(ProtocolError::ReceiptMismatch);
            }
        }
        Ok(())
    }
}
