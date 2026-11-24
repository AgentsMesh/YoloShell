use crate::{
    ConnectorError, FIRST_CONNECTOR_REVISION, FIRST_PUBLICATION_SEQUENCE, MAX_PUBLICATION_SEQUENCE,
};
use serde::{Deserialize, Deserializer, Serializer, de::Error};
use uuid::Uuid;

pub(crate) fn request_id(value: Uuid) -> Result<(), ConnectorError> {
    if value.is_nil() {
        Err(ConnectorError::InvalidRequest)
    } else {
        Ok(())
    }
}

pub(crate) fn revision(value: u64) -> Result<(), ConnectorError> {
    if value < FIRST_CONNECTOR_REVISION {
        Err(ConnectorError::InvalidRequest)
    } else {
        Ok(())
    }
}

pub(crate) fn sequence(value: u64) -> Result<(), ConnectorError> {
    if (FIRST_PUBLICATION_SEQUENCE..=MAX_PUBLICATION_SEQUENCE).contains(&value) {
        Ok(())
    } else {
        Err(ConnectorError::Sequence)
    }
}

pub(crate) fn deserialize_sequence<'de, D: Deserializer<'de>>(input: D) -> Result<u64, D::Error> {
    let value = u64::deserialize(input)?;
    sequence(value).map_err(D::Error::custom)?;
    Ok(value)
}

pub(crate) fn deserialize_revision<'de, D: Deserializer<'de>>(input: D) -> Result<u64, D::Error> {
    let value = u64::deserialize(input)?;
    revision(value).map_err(D::Error::custom)?;
    Ok(value)
}

pub(crate) mod id {
    use super::*;

    pub fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<Uuid, D::Error> {
        let text = String::deserialize(input)?;
        let value = yolo_crypto::serde_uuid::parse_canonical(&text)
            .ok_or_else(|| D::Error::custom(ConnectorError::InvalidRequest))?;
        request_id(value).map_err(D::Error::custom)?;
        Ok(value)
    }

    pub fn serialize<S: Serializer>(value: &Uuid, output: S) -> Result<S::Ok, S::Error> {
        request_id(*value).map_err(serde::ser::Error::custom)?;
        output.serialize_str(&value.to_string())
    }
}
