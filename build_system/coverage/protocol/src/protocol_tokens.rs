use crate::{
    HEADER_PREFIX, INSTANCE_INVENTORY_PREFIX, INSTANCE_MEMBER_PREFIX, MetricKind, ORDINAL_PREFIX,
    SOURCE_INVENTORY_PREFIX,
};

pub fn source_inventory_token(inventory: &str, members: u64) -> String {
    format!("{SOURCE_INVENTORY_PREFIX}{inventory}/{members}")
}

pub fn instance_inventory_token(inventory: &str, members: u64) -> String {
    format!("{INSTANCE_INVENTORY_PREFIX}{inventory}/{members}")
}

pub fn instance_member_token(inventory: &str, instance: &str) -> String {
    format!("{INSTANCE_MEMBER_PREFIX}{inventory}/{instance}")
}

pub fn header_token(
    kind: MetricKind,
    line: u64,
    column: u64,
    identity: &str,
    signature: &str,
    vector_len: u64,
    found: u64,
) -> String {
    format!(
        "{HEADER_PREFIX}{}/{line}/{column}/{identity}/{signature}/{vector_len}/{found}",
        kind.path()
    )
}

pub fn ordinal_token(
    kind: MetricKind,
    line: u64,
    column: u64,
    identity: &str,
    signature: &str,
    ordinal: u64,
) -> String {
    format!(
        "{ORDINAL_PREFIX}{}/{line}/{column}/{identity}/{signature}/{ordinal}",
        kind.path()
    )
}
