use super::constants::{event_names, field_names, permission_modes, test_values, values};
use super::{CodexHookInput, decode};
use serde_json::{Map, Value};

pub(super) fn input(event: &str) -> CodexHookInput {
    decode_value(input_value(event)).unwrap()
}

pub(super) fn decode_value(value: Value) -> anyhow::Result<CodexHookInput> {
    decode(&serde_json::to_vec(&value).unwrap())
}

pub(super) fn input_value(event: &str) -> Value {
    let mut value = common(event);
    match event {
        event_names::SESSION_START => {
            model_and_permission(&mut value);
            insert(&mut value, field_names::SOURCE, values::STARTUP);
        }
        event_names::SESSION_END => insert(&mut value, field_names::REASON, values::OTHER),
        event_names::USER_PROMPT_SUBMIT => {
            turn(&mut value);
            model_and_permission(&mut value);
            insert(&mut value, field_names::PROMPT, test_values::PROMPT);
        }
        event_names::PRE_TOOL_USE => {
            tool(&mut value);
            insert(
                &mut value,
                field_names::TOOL_USE_ID,
                test_values::TOOL_USE_ID,
            );
        }
        event_names::PERMISSION_REQUEST => tool(&mut value),
        event_names::POST_TOOL_USE => {
            tool(&mut value);
            insert(
                &mut value,
                field_names::TOOL_USE_ID,
                test_values::TOOL_USE_ID,
            );
            value.insert(
                field_names::TOOL_RESPONSE.to_owned(),
                Value::Object(Map::new()),
            );
        }
        event_names::PRE_COMPACT | event_names::POST_COMPACT => {
            turn(&mut value);
            insert(&mut value, field_names::MODEL, test_values::MODEL);
            insert(&mut value, field_names::TRIGGER, values::MANUAL);
        }
        event_names::SUBAGENT_START => {
            turn(&mut value);
            model_and_permission(&mut value);
            agent(&mut value);
        }
        event_names::SUBAGENT_STOP => {
            turn(&mut value);
            model_and_permission(&mut value);
            agent(&mut value);
            value.insert(field_names::AGENT_TRANSCRIPT_PATH.to_owned(), Value::Null);
            value.insert(field_names::LAST_ASSISTANT_MESSAGE.to_owned(), Value::Null);
            value.insert(field_names::STOP_HOOK_ACTIVE.to_owned(), Value::Bool(false));
        }
        event_names::STOP => {
            turn(&mut value);
            model_and_permission(&mut value);
            value.insert(field_names::LAST_ASSISTANT_MESSAGE.to_owned(), Value::Null);
            value.insert(field_names::STOP_HOOK_ACTIVE.to_owned(), Value::Bool(false));
        }
        _ => {}
    }
    Value::Object(value)
}

fn common(event: &str) -> Map<String, Value> {
    let mut value = Map::new();
    insert(
        &mut value,
        field_names::SESSION_ID,
        super::TEST_PROVIDER_SESSION,
    );
    value.insert(field_names::TRANSCRIPT_PATH.to_owned(), Value::Null);
    insert(&mut value, field_names::CWD, super::TEST_CWD);
    insert(&mut value, field_names::EVENT_NAME, event);
    value
}

fn turn(value: &mut Map<String, Value>) {
    insert(value, field_names::TURN_ID, super::TEST_TURN);
}

fn model_and_permission(value: &mut Map<String, Value>) {
    insert(value, field_names::MODEL, test_values::MODEL);
    insert(
        value,
        field_names::PERMISSION_MODE,
        permission_modes::DEFAULT,
    );
}

fn tool(value: &mut Map<String, Value>) {
    turn(value);
    model_and_permission(value);
    insert(value, field_names::TOOL_NAME, test_values::TOOL_NAME);
    value.insert(
        field_names::TOOL_INPUT.to_owned(),
        Value::Object(Map::new()),
    );
}

fn agent(value: &mut Map<String, Value>) {
    insert(value, field_names::AGENT_ID, test_values::AGENT_ID);
    insert(value, field_names::AGENT_TYPE, test_values::AGENT_TYPE);
}

fn insert(value: &mut Map<String, Value>, field: &str, content: &str) {
    value.insert(field.to_owned(), Value::String(content.to_owned()));
}
