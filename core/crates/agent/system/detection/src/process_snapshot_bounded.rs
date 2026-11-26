use super::limits::{
    MAX_PROCESS_SNAPSHOT_ARGV_ITEMS, MAX_PROCESS_SNAPSHOT_ENVIRONMENT_ITEMS,
    MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS, MAX_PROCESS_SNAPSHOT_PROCESSES,
};
use super::wire::{ObservedProcessWire, UnixPathWire};
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

const COLLECTION_LIMIT: &str = "process snapshot collection exceeds its item budget";
const DUPLICATE_MAP_KEY: &str = "process snapshot map contains a duplicate key";

struct BoundedVec<T, const LIMIT: usize>(Vec<T>);

impl<'de, T, const LIMIT: usize> Deserialize<'de> for BoundedVec<T, LIMIT>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BoundedVecVisitor::<T, LIMIT>(PhantomData))
    }
}

struct BoundedVecVisitor<T, const LIMIT: usize>(PhantomData<T>);

impl<'de, T, const LIMIT: usize> Visitor<'de> for BoundedVecVisitor<T, LIMIT>
where
    T: Deserialize<'de>,
{
    type Value = BoundedVec<T, LIMIT>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a process snapshot sequence within its item budget")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let count = sequence.size_hint().unwrap_or_default();
        if count > LIMIT {
            return Err(serde::de::Error::custom(COLLECTION_LIMIT));
        }
        let mut values = Vec::with_capacity(count.min(LIMIT));
        while let Some(value) = sequence.next_element()? {
            if values.len() == LIMIT {
                return Err(serde::de::Error::custom(COLLECTION_LIMIT));
            }
            values.push(value);
        }
        Ok(BoundedVec(values))
    }
}

struct BoundedMap<K, V, const LIMIT: usize>(BTreeMap<K, V>);

impl<'de, K, V, const LIMIT: usize> Deserialize<'de> for BoundedMap<K, V, LIMIT>
where
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(BoundedMapVisitor::<K, V, LIMIT>(PhantomData))
    }
}

struct BoundedMapVisitor<K, V, const LIMIT: usize>(PhantomData<(K, V)>);

impl<'de, K, V, const LIMIT: usize> Visitor<'de> for BoundedMapVisitor<K, V, LIMIT>
where
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    type Value = BoundedMap<K, V, LIMIT>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a process snapshot map within its item budget")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let count = map.size_hint().unwrap_or_default();
        if count > LIMIT {
            return Err(serde::de::Error::custom(COLLECTION_LIMIT));
        }
        let mut values = BTreeMap::new();
        while let Some((key, value)) = map.next_entry()? {
            if values.len() == LIMIT {
                return Err(serde::de::Error::custom(COLLECTION_LIMIT));
            }
            if values.insert(key, value).is_some() {
                return Err(serde::de::Error::custom(DUPLICATE_MAP_KEY));
            }
        }
        Ok(BoundedMap(values))
    }
}

pub(super) fn processes<'de, D>(deserializer: D) -> Result<Vec<ObservedProcessWire>, D::Error>
where
    D: Deserializer<'de>,
{
    BoundedVec::<_, MAX_PROCESS_SNAPSHOT_PROCESSES>::deserialize(deserializer)
        .map(|values| values.0)
}

pub(super) fn argv<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    BoundedVec::<_, MAX_PROCESS_SNAPSHOT_ARGV_ITEMS>::deserialize(deserializer)
        .map(|values| values.0)
}

pub(super) fn environment<'de, D>(
    deserializer: D,
) -> Result<Option<BTreeMap<String, String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<BoundedMap<_, _, MAX_PROCESS_SNAPSHOT_ENVIRONMENT_ITEMS>>::deserialize(deserializer)
        .map(|values| values.map(|value| value.0))
}

pub(super) fn evidence<'de, D>(deserializer: D) -> Result<Option<Vec<UnixPathWire>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<BoundedVec<_, MAX_PROCESS_SNAPSHOT_EVIDENCE_ITEMS>>::deserialize(deserializer)
        .map(|values| values.map(|value| value.0))
}
