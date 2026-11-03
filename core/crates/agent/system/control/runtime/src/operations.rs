use crate::{Completion, ControlRuntime, JournalError, Receipt, Reservation, ReservationResult};
use uuid::Uuid;
use yolo_crypto::Sha256Digest;
use yolo_terminal_model::RuntimeRecord;

impl ControlRuntime {
    pub fn lookup(
        &self,
        current: &RuntimeRecord,
        request: Uuid,
        digest: Sha256Digest,
    ) -> Result<Option<Receipt>, JournalError> {
        self.verify(current)?;
        let result = self.journal.lookup(request, digest);
        let value = self.retain_failure(result)?;
        self.verify(current)?;
        Ok(value)
    }
    pub fn reserve(
        &mut self,
        current: &RuntimeRecord,
        request: Uuid,
        digest: Sha256Digest,
    ) -> Result<ReservationResult, JournalError> {
        self.verify(current)?;
        let result = self.journal.reserve(request, digest);
        self.after_write(current, result)
    }
    pub fn complete(
        &mut self,
        current: &RuntimeRecord,
        reservation: Reservation,
        completion: Completion,
    ) -> Result<Receipt, JournalError> {
        self.verify(current)?;
        let result = self.journal.complete(reservation, completion);
        self.after_write(current, result)
    }
}
