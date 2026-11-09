use crate::{
    INSTANCE_IDENTITY_DOMAIN, INSTANCE_INVENTORY_DOMAIN, MetricShape, SHA256_HEX_LENGTH,
    SOURCE_INVENTORY_DOMAIN,
};
use sha2::{Digest, Sha256};
use std::fmt::Write;

const HEX_ALPHABET: &[u8] = b"0123456789abcdef";

pub fn encode_fields(fields: &[&str]) -> Result<String, String> {
    let mut bytes = Vec::new();
    for field in fields {
        append_field(&mut bytes, field)?;
    }
    Ok(digest_bytes(&bytes))
}

pub fn source_inventory_digest(sources: &[&str]) -> Result<String, String> {
    inventory_digest(SOURCE_INVENTORY_DOMAIN, sources)
}

pub fn instance_inventory_digest(instances: &[&str]) -> Result<String, String> {
    inventory_digest(INSTANCE_INVENTORY_DOMAIN, instances)
}

pub fn instance_digest(
    line: u64,
    column: u64,
    identity: &str,
    signature: &str,
    shapes: &[MetricShape; 4],
) -> String {
    let mut bytes = Vec::new();
    append_field(&mut bytes, INSTANCE_IDENTITY_DOMAIN).expect("static digest domain is valid");
    bytes.extend_from_slice(&line.to_be_bytes());
    bytes.extend_from_slice(&column.to_be_bytes());
    append_field(&mut bytes, identity).expect("SHA-256 identity length fits u64");
    append_field(&mut bytes, signature).expect("SHA-256 signature length fits u64");
    for shape in shapes {
        bytes.extend_from_slice(&shape.vector_len.to_be_bytes());
        bytes.extend_from_slice(&shape.found.to_be_bytes());
    }
    digest_bytes(&bytes)
}

pub fn digest_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(SHA256_HEX_LENGTH);
    for byte in Sha256::digest(bytes) {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

fn inventory_digest(domain: &str, members: &[&str]) -> Result<String, String> {
    if members.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("coverage inventory members are not strictly ordered".to_owned());
    }
    let mut bytes = Vec::new();
    append_field(&mut bytes, domain)?;
    for member in members {
        append_field(&mut bytes, member)?;
    }
    Ok(digest_bytes(&bytes))
}

fn append_field(bytes: &mut Vec<u8>, field: &str) -> Result<(), String> {
    let length = u64::try_from(field.len()).map_err(|_| "digest field is too long")?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(field.as_bytes());
    Ok(())
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|byte| HEX_ALPHABET.contains(&byte))
}
