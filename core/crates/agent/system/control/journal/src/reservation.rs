use crate::Receipt;
use uuid::Uuid;
use yolo_crypto::Sha256Digest;

/// A one-use capability to finish a newly persisted intent. Not Clone or serializable.
#[derive(Debug)]
pub struct Reservation {
    pub(crate) owner: Uuid,
    pub(crate) request_id: Uuid,
    pub(crate) digest: Sha256Digest,
}
impl Reservation {
    pub fn request_id(&self) -> Uuid {
        self.request_id
    }
}

#[derive(Debug)]
pub enum ReservationResult {
    New(Reservation),
    Existing(Receipt),
}
