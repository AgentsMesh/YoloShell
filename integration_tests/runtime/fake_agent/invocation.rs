use crate::fake_agent_constants as constants;
#[cfg(test)]
use serde_json::Value;
use serde_json::json;

#[cfg(test)]
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Invocation {
    pub(crate) runtime_epoch: String,
    pub(crate) arguments: Vec<String>,
}

pub(crate) fn encode(runtime_epoch: &str, arguments: &[String]) -> serde_json::Result<Vec<u8>> {
    let value = json!({
        (constants::INVOCATION_RUNTIME_EPOCH_KEY): runtime_epoch,
        (constants::INVOCATION_ARGUMENTS_KEY): arguments,
    });
    let mut encoded = serde_json::to_vec(&value)?;
    encoded.push(constants::INVOCATION_LINE_ENDING);
    Ok(encoded)
}

#[cfg(test)]
pub(crate) fn decode(line: &str) -> Result<Invocation, &'static str> {
    let value: Value =
        serde_json::from_str(line).map_err(|_| constants::INVOCATION_JSON_INVALID)?;
    let object = value
        .as_object()
        .filter(|object| object.len() == constants::INVOCATION_RECORD_FIELD_COUNT)
        .ok_or(constants::INVOCATION_OBJECT_INVALID)?;
    let runtime_epoch = object
        .get(constants::INVOCATION_RUNTIME_EPOCH_KEY)
        .and_then(Value::as_str)
        .ok_or(constants::INVOCATION_RUNTIME_EPOCH_INVALID)?;
    let arguments = object
        .get(constants::INVOCATION_ARGUMENTS_KEY)
        .and_then(Value::as_array)
        .ok_or(constants::INVOCATION_ARGUMENTS_INVALID)?
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or(constants::INVOCATION_ARGUMENTS_INVALID)?;
    Ok(Invocation {
        runtime_epoch: runtime_epoch.to_owned(),
        arguments: arguments.into_iter().map(str::to_owned).collect(),
    })
}
