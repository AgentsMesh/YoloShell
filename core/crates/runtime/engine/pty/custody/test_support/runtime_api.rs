use crate::pty_custody::{CustodyDurableLease, CustodyDurableRecord, CustodyUnixStream};

pub fn custody_offer(
    request: &crate::CustodyRequest,
    descriptor_set: crate::CustodyDescriptorSet,
    digest: crate::sha256_digest::Sha256Digest,
) -> crate::CustodyOffer {
    crate::CustodyOffer::new(request, descriptor_set, digest)
}

pub fn retire_without_evidence(
    record: &CustodyDurableRecord,
    lease: CustodyDurableLease,
) -> Option<CustodyDurableRecord> {
    record.retire(lease)
}

pub async fn write_truncated_frame<T: serde::Serialize>(
    stream: &mut CustodyUnixStream,
    value: &T,
) -> anyhow::Result<()> {
    stream.write_truncated_frame(value).await
}
