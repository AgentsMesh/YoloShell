use anyhow::{Result, ensure};
use serde_json::Value;
use std::collections::BTreeSet;
use yolo_wire::field::PHASE as PHASE_FIELD;

const VERSION_FIELD: &str = "version";
const HOST_ID_FIELD: &str = "hostID";
const MUTATION_ID_FIELD: &str = "mutationID";
const PROFILE_FIELD: &str = "profile";
const PROFILE_SHA256_FIELD: &str = "profileSHA256";
const PREVIOUS_PROFILE_FIELD: &str = "previousProfile";
const PREVIOUS_PROFILE_SHA256_FIELD: &str = "previousProfileSHA256";
const BLOCKED_PHASE: &str = "blocked";
const COMMITTED_PHASE: &str = "committed";
const DELETED_PHASE: &str = "deleted";

pub(super) fn validate(value: &Value) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("remote host authority is not an object"))?;
    ensure!(
        object.values().all(|value| !value.is_null()),
        "remote host authority contains an explicit null"
    );
    let phase = object
        .get(PHASE_FIELD)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("remote host authority phase is missing"))?;
    let mut expected = vec![VERSION_FIELD, HOST_ID_FIELD, PHASE_FIELD];
    match phase {
        BLOCKED_PHASE => {
            expected.push(MUTATION_ID_FIELD);
            if object.contains_key(PREVIOUS_PROFILE_FIELD)
                || object.contains_key(PREVIOUS_PROFILE_SHA256_FIELD)
            {
                expected.extend([PREVIOUS_PROFILE_FIELD, PREVIOUS_PROFILE_SHA256_FIELD]);
            }
        }
        COMMITTED_PHASE => expected.extend([PROFILE_FIELD, PROFILE_SHA256_FIELD]),
        DELETED_PHASE => {}
        _ => anyhow::bail!("remote host authority phase is invalid"),
    }
    let actual: BTreeSet<_> = object.keys().map(String::as_str).collect();
    ensure!(
        actual == expected.into_iter().collect(),
        "remote host authority fields do not match its phase"
    );
    Ok(())
}
