use crate::{
    Completion, Journal, JournalError, Receipt, Reservation, ReservationResult,
    constants::MAX_RECEIPTS, record::Entry,
};
use uuid::Uuid;
use yolo_crypto::Sha256Digest;

impl Journal {
    pub fn lookup(
        &self,
        request_id: Uuid,
        digest: Sha256Digest,
    ) -> Result<Option<Receipt>, JournalError> {
        let result = (|| {
            validate_id(request_id)?;
            self.verify_inner()?;
            self.record.lookup(request_id, digest)
        })();
        self.retain_failure(result)
    }

    /// Only New permits one attempt. Existing Unknown is never a retry permission.
    /// The caller supplies the canonical request digest, excluding ephemeral transport IDs.
    pub fn reserve(
        &mut self,
        request_id: Uuid,
        digest: Sha256Digest,
    ) -> Result<ReservationResult, JournalError> {
        let result = (|| {
            validate_id(request_id)?;
            self.verify_inner()?;
            if let Some(receipt) = self.record.lookup(request_id, digest)? {
                return Ok(ReservationResult::Existing(receipt));
            }
            if self.record.entries.len() >= MAX_RECEIPTS {
                return Err(JournalError::Capacity);
            }
            let mut next = self.record.clone();
            next.entries.push(Entry {
                request_id,
                digest,
                receipt: Receipt::Unknown,
            });
            self.commit(next)?;
            Ok(ReservationResult::New(Reservation {
                owner: self.owner,
                request_id,
                digest,
            }))
        })();
        self.retain_failure(result)
    }

    /// Consumes the unique reservation. A failed commit remains unknown; do not repeat effects.
    pub fn complete(
        &mut self,
        reservation: Reservation,
        outcome: Completion,
    ) -> Result<Receipt, JournalError> {
        let result = (|| {
            if reservation.owner != self.owner {
                return Err(JournalError::ForeignReservation);
            }
            self.verify_inner()?;
            if self
                .record
                .lookup(reservation.request_id, reservation.digest)?
                != Some(Receipt::Unknown)
            {
                return Err(JournalError::Conflict);
            }
            let mut next = self.record.clone();
            let entry = next
                .entries
                .iter_mut()
                .find(|entry| entry.request_id == reservation.request_id)
                .ok_or(JournalError::Conflict)?;
            let receipt = outcome.into();
            entry.receipt = receipt;
            self.commit(next)?;
            Ok(receipt)
        })();
        self.retain_failure(result)
    }
}

fn validate_id(id: Uuid) -> Result<(), JournalError> {
    if id.is_nil() {
        Err(JournalError::InvalidRequest)
    } else {
        Ok(())
    }
}
