use serde::{Deserialize, Deserializer};

pub(super) fn deserialize<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<String>::deserialize(deserializer).map(Some)
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/runtime_capability_wire_inline_tests.rs"]
mod tests;
