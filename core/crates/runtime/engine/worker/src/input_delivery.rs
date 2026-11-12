use crate::model::WriterCapability;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

const SHA256_OUTPUT_BYTES: usize = 32;
pub(super) const CAPACITY_EXHAUSTED_DETAIL: &str =
    "terminal input deduplication capacity is exhausted";
const RESERVED_DELIVERY_MISSING_DETAIL: &str = "reserved input delivery is missing";
type ContentDigest = [u8; SHA256_OUTPUT_BYTES];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DeliveryKey {
    capability: ContentDigest,
    input_id: Uuid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CachedInputDelivery {
    Delivered,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CachedInputLookup {
    Missing,
    Matching(CachedInputDelivery),
    PayloadMismatch,
}

#[derive(Debug, PartialEq, Eq)]
struct CachedInput {
    delivery: CachedInputDelivery,
    digest: ContentDigest,
}

#[derive(Debug, Default)]
pub(super) struct InputDeliveryCache {
    order: VecDeque<DeliveryKey>,
    capability_order: VecDeque<ContentDigest>,
    entries: HashMap<DeliveryKey, CachedInput>,
    latest_by_capability: HashMap<ContentDigest, Uuid>,
}

impl InputDeliveryCache {
    pub(super) fn get(
        &self,
        writer_capability: &WriterCapability,
        input_id: Uuid,
        data: &[u8],
    ) -> CachedInputLookup {
        let key = delivery_key(writer_capability, input_id);
        let Some(cached) = self.entries.get(&key) else {
            return CachedInputLookup::Missing;
        };
        if cached.digest == payload_digest(data) {
            CachedInputLookup::Matching(cached.delivery)
        } else {
            CachedInputLookup::PayloadMismatch
        }
    }

    pub(super) fn reserve(
        &mut self,
        writer_capability: &WriterCapability,
        input_id: Uuid,
        data: &[u8],
        capacity: usize,
    ) -> bool {
        let key = delivery_key(writer_capability, input_id);
        if let Some(cached) = self.entries.get(&key) {
            return cached.digest == payload_digest(data);
        }
        if !self.make_room_for_capability(capacity) {
            return false;
        }
        let previous = self.latest_by_capability.insert(key.capability, input_id);
        if let Some(previous_id) = previous {
            let previous_key = DeliveryKey {
                capability: key.capability,
                input_id: previous_id,
            };
            if self.entries.contains_key(&previous_key) {
                self.order.push_back(previous_key);
            }
        } else {
            self.capability_order.push_back(key.capability);
        }
        self.entries.insert(
            key,
            CachedInput {
                delivery: CachedInputDelivery::Unknown,
                digest: payload_digest(data),
            },
        );
        self.trim(capacity);
        true
    }

    pub(super) fn mark_delivered(&mut self, writer_capability: &WriterCapability, input_id: Uuid) {
        self.entries
            .get_mut(&delivery_key(writer_capability, input_id))
            .expect(RESERVED_DELIVERY_MISSING_DETAIL)
            .delivery = CachedInputDelivery::Delivered;
    }

    #[cfg(test)]
    pub(super) fn record(
        &mut self,
        writer_capability: &WriterCapability,
        input_id: Uuid,
        data: &[u8],
        delivery: CachedInputDelivery,
        capacity: usize,
    ) {
        if self.reserve(writer_capability, input_id, data, capacity)
            && delivery == CachedInputDelivery::Delivered
        {
            self.mark_delivered(writer_capability, input_id);
        }
    }

    pub(super) fn clear_capability(&mut self, writer_capability: &WriterCapability) {
        let capability = capability_digest(writer_capability);
        self.latest_by_capability.remove(&capability);
        self.capability_order.retain(|item| *item != capability);
        self.order.retain(|key| key.capability != capability);
        self.entries.retain(|key, _| key.capability != capability);
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    pub(super) fn capability_len(&self) -> usize {
        self.latest_by_capability.len()
    }

    #[cfg(test)]
    pub(super) fn evictable_len(&self) -> usize {
        self.order.len()
    }

    fn make_room_for_capability(&mut self, capacity: usize) -> bool {
        while self.entries.len() >= capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
                continue;
            }
            let Some(capability) = self.capability_order.pop_front() else {
                return false;
            };
            if let Some(input_id) = self.latest_by_capability.remove(&capability) {
                self.entries.remove(&DeliveryKey {
                    capability,
                    input_id,
                });
            }
        }
        true
    }

    fn trim(&mut self, capacity: usize) {
        while self.entries.len() > capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
                continue;
            }
            let Some(capability) = self.capability_order.pop_front() else {
                break;
            };
            if let Some(input_id) = self.latest_by_capability.remove(&capability) {
                self.entries.remove(&DeliveryKey {
                    capability,
                    input_id,
                });
            }
        }
    }
}

fn delivery_key(writer_capability: &WriterCapability, input_id: Uuid) -> DeliveryKey {
    DeliveryKey {
        capability: capability_digest(writer_capability),
        input_id,
    }
}

fn capability_digest(writer_capability: &WriterCapability) -> ContentDigest {
    Sha256::digest(writer_capability.as_bytes()).into()
}

fn payload_digest(data: &[u8]) -> ContentDigest {
    Sha256::digest(data).into()
}
