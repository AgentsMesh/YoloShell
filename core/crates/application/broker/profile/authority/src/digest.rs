use anyhow::Result;
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

const PROFILES_FIELD: &str = "profiles";
const VERSION_FIELD: &str = "version";
const PROFILE_DOCUMENT_VERSION: u64 = 1;

pub(super) fn profile_sha256(profile: &Value) -> Result<String> {
    let mut wrapper = Map::new();
    wrapper.insert(PROFILES_FIELD.into(), Value::Array(vec![profile.clone()]));
    wrapper.insert(
        VERSION_FIELD.into(),
        Value::Number(Number::from(PROFILE_DOCUMENT_VERSION)),
    );
    let canonical = recursively_sorted(Value::Object(wrapper));
    let digest = Sha256::digest(serde_json::to_vec(&canonical)?);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn recursively_sorted(value: Value) -> Value {
    match value {
        Value::Object(values) => {
            let mut entries: Vec<_> = values.into_iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = Map::new();
            for (key, value) in entries {
                sorted.insert(key, recursively_sorted(value));
            }
            Value::Object(sorted)
        }
        Value::Array(values) => Value::Array(values.into_iter().map(recursively_sorted).collect()),
        primitive => primitive,
    }
}
