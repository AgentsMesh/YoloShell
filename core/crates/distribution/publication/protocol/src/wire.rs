use crate::{InspectionReceipt, PublicationReceipt, PublicationRequest, Selection, constants::*};
use anyhow::{Result, ensure};
use serde::{Serialize, de::DeserializeOwned};

pub(crate) fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
}

pub fn decode_request(bytes: &[u8]) -> Result<PublicationRequest> {
    decode(bytes)
}
pub fn decode_receipt(bytes: &[u8]) -> Result<PublicationReceipt> {
    decode(bytes)
}
pub fn decode_inspection(bytes: &[u8]) -> Result<InspectionReceipt> {
    decode(bytes)
}
pub fn decode_selection(bytes: &[u8]) -> Result<Selection> {
    ensure!(bytes.len() <= MAX_PUBLICATION_BYTES, PUBLICATION_LIMIT);
    serde_json::from_slice(bytes).map_err(Into::into)
}
pub fn encode_request(value: &PublicationRequest) -> Result<Vec<u8>> {
    encode(value)
}
pub fn encode_receipt(value: &PublicationReceipt) -> Result<Vec<u8>> {
    encode(value)
}
pub fn encode_inspection(value: &InspectionReceipt) -> Result<Vec<u8>> {
    encode(value)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    ensure!(bytes.len() <= MAX_PUBLICATION_BYTES, PUBLICATION_LIMIT);
    yolo_capnp_codec::decode(bytes).map_err(Into::into)
}
fn encode(value: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = yolo_capnp_codec::encode(value)?;
    ensure!(bytes.len() <= MAX_PUBLICATION_BYTES, PUBLICATION_LIMIT);
    Ok(bytes)
}

pub fn encode_request_argument(value: &PublicationRequest) -> Result<String> {
    yolo_capnp_argument::encode(value, MAX_PUBLICATION_BYTES).map_err(Into::into)
}
pub fn decode_request_argument(value: &str) -> Result<PublicationRequest> {
    yolo_capnp_argument::decode(value, MAX_PUBLICATION_BYTES).map_err(Into::into)
}
