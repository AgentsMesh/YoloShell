use crate::{
    constants::*,
    fields::{Field, FieldId, Value},
};
use anyhow::{Result, ensure};
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};

pub(crate) fn key_text(identity: PeerIdentity) -> String {
    identity
        .key()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(crate) fn identity(fields: &[Field]) -> Result<PeerIdentity> {
    let hub = text(fields, FieldId::Hub)?.parse()?;
    let key = text(fields, FieldId::Key)?;
    ensure!(
        key.len() == ED25519_PUBLIC_KEY_BYTES * HEX_DIGITS_PER_BYTE && key.is_ascii(),
        INVALID
    );
    let mut bytes = [u8::default(); ED25519_PUBLIC_KEY_BYTES];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let offset = index * HEX_DIGITS_PER_BYTE;
        *byte = u8::from_str_radix(&key[offset..offset + HEX_DIGITS_PER_BYTE], HEX_RADIX)?;
    }
    Ok(PeerIdentity::new(hub, bytes)?)
}
pub(crate) fn text(fields: &[Field], id: FieldId) -> Result<&str> {
    raw_text(fields, id).map(str::trim)
}
pub(crate) fn raw_text(fields: &[Field], id: FieldId) -> Result<&str> {
    match fields
        .iter()
        .find(|field| field.id == id)
        .map(|field| &field.value)
    {
        Some(Value::Text { text, .. }) => Ok(text),
        _ => anyhow::bail!(INVALID),
    }
}
pub(crate) fn flag(fields: &[Field], id: FieldId) -> Result<bool> {
    match fields
        .iter()
        .find(|field| field.id == id)
        .map(|field| &field.value)
    {
        Some(Value::Toggle(value)) => Ok(*value),
        _ => anyhow::bail!(INVALID),
    }
}
pub(crate) fn number(fields: &[Field], id: FieldId) -> Result<u8> {
    match fields
        .iter()
        .find(|field| field.id == id)
        .map(|field| &field.value)
    {
        Some(Value::Stepper(value)) => Ok(*value),
        _ => anyhow::bail!(INVALID),
    }
}
