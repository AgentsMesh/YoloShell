use crate::protocol_digest::valid_digest;
use crate::{
    FunctionToken, HEADER_PREFIX, INSTANCE_INVENTORY_PREFIX, INSTANCE_MEMBER_PREFIX, MARKER_TOKEN,
    MAXIMUM_INSTANCE_VECTOR_LENGTH, MAXIMUM_SOURCE_POSITION, MINIMUM_SOURCE_POSITION, MetricKind,
    ORDINAL_PREFIX, SOURCE_INVENTORY_PREFIX, TOKEN_ROOT,
};

const INVENTORY_FIELD_COUNT: usize = 2;
const HEADER_FIELD_COUNT: usize = 7;
const ORDINAL_FIELD_COUNT: usize = 6;

pub fn parse_token(name: &str) -> Result<Option<FunctionToken<'_>>, String> {
    if name == MARKER_TOKEN {
        return Ok(Some(FunctionToken::Marker));
    }
    if let Some(payload) = name.strip_prefix(SOURCE_INVENTORY_PREFIX) {
        let fields = split(payload, INVENTORY_FIELD_COUNT, name)?;
        return Ok(Some(FunctionToken::SourceInventory {
            inventory: digest(fields[0], "source inventory")?,
            members: bounded_nonzero(fields[1], "source member count")?,
        }));
    }
    if let Some(payload) = name.strip_prefix(INSTANCE_INVENTORY_PREFIX) {
        let fields = split(payload, INVENTORY_FIELD_COUNT, name)?;
        return Ok(Some(FunctionToken::InstanceInventory {
            inventory: digest(fields[0], "instance inventory")?,
            members: bounded(fields[1], "instance member count")?,
        }));
    }
    if let Some(payload) = name.strip_prefix(INSTANCE_MEMBER_PREFIX) {
        let fields = split(payload, INVENTORY_FIELD_COUNT, name)?;
        return Ok(Some(FunctionToken::InstanceMember {
            inventory: digest(fields[0], "instance inventory")?,
            instance: digest(fields[1], "instance")?,
        }));
    }
    if let Some(payload) = name.strip_prefix(HEADER_PREFIX) {
        return header(payload, name).map(Some);
    }
    if let Some(payload) = name.strip_prefix(ORDINAL_PREFIX) {
        return ordinal(payload, name).map(Some);
    }
    if name.starts_with(TOKEN_ROOT) {
        return Err(format!("unknown synthetic function token {name:?}"));
    }
    Ok(None)
}

fn header<'a>(payload: &'a str, whole: &str) -> Result<FunctionToken<'a>, String> {
    let fields = split(payload, HEADER_FIELD_COUNT, whole)?;
    let vector_len = bounded(fields[5], "vector length")?;
    let found = bounded(fields[6], "denominator")?;
    if found > vector_len {
        return Err("synthetic denominator exceeds vector length".to_owned());
    }
    Ok(FunctionToken::Header {
        kind: kind(fields[0])?,
        line: position(fields[1], "line")?,
        column: position(fields[2], "column")?,
        identity: digest(fields[3], "identity")?,
        signature: digest(fields[4], "signature")?,
        vector_len,
        found,
    })
}

fn ordinal<'a>(payload: &'a str, whole: &str) -> Result<FunctionToken<'a>, String> {
    let fields = split(payload, ORDINAL_FIELD_COUNT, whole)?;
    Ok(FunctionToken::Ordinal {
        kind: kind(fields[0])?,
        line: position(fields[1], "line")?,
        column: position(fields[2], "column")?,
        identity: digest(fields[3], "identity")?,
        signature: digest(fields[4], "signature")?,
        ordinal: bounded(fields[5], "ordinal")?,
    })
}

fn split<'a>(value: &'a str, count: usize, whole: &str) -> Result<Vec<&'a str>, String> {
    let fields = value.split('/').collect::<Vec<_>>();
    (fields.len() == count)
        .then_some(fields)
        .ok_or_else(|| format!("malformed synthetic function token {whole:?}"))
}

fn kind(value: &str) -> Result<MetricKind, String> {
    MetricKind::from_path(value).ok_or_else(|| format!("unknown synthetic metric {value:?}"))
}

fn number(value: &str, label: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid synthetic {label} {value:?}"))
}

fn bounded(value: &str, label: &str) -> Result<u64, String> {
    let value = number(value, label)?;
    (value <= MAXIMUM_INSTANCE_VECTOR_LENGTH)
        .then_some(value)
        .ok_or_else(|| format!("synthetic {label} exceeds the protocol limit"))
}

fn bounded_nonzero(value: &str, label: &str) -> Result<u64, String> {
    let value = bounded(value, label)?;
    (value > 0)
        .then_some(value)
        .ok_or_else(|| format!("synthetic {label} is zero"))
}

fn position(value: &str, label: &str) -> Result<u64, String> {
    let value = number(value, label)?;
    (MINIMUM_SOURCE_POSITION..=MAXIMUM_SOURCE_POSITION)
        .contains(&value)
        .then_some(value)
        .ok_or_else(|| format!("invalid synthetic {label}"))
}

fn digest<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    valid_digest(value)
        .then_some(value)
        .ok_or_else(|| format!("invalid synthetic {label}"))
}
