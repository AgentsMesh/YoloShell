use crate::constants::*;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use yolo_hook_installation_contract::{HookEventSpec, HookInstallSpec};

pub fn callback_command(spec: &HookInstallSpec) -> String {
    let executable = yolo_agent_runtime::NATIVE_HOOK_EXECUTABLE_ENV;
    let home = yolo_agent_runtime::NATIVE_HOOK_HOME_ENV;
    let arguments = spec.hook_arguments;
    let argument = yolo_cli::constants::HOME_ARGUMENT;
    let role = yolo_cli::ProcessRole::Host.arguments().join(" ");
    format!(
        "if [ -n \"${{{home}:-}}\" ] && [ -n \"${{{executable}:-}}\" ]; then exec \"${{{executable}}}\" {role} {arguments} {argument} \"${{{home}}}\"; fi"
    )
}

pub fn merge(spec: &HookInstallSpec, existing: &Value) -> Result<Value> {
    validate(existing)?;
    ensure!(!restricted(spec, existing), CONFIG_DISABLED);
    let mut result = existing.clone();
    let root = result
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!(INVALID_CONFIG))?;
    let hooks = root.entry(HOOKS).or_insert_with(|| json!({}));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!(INVALID_CONFIG))?;
    for event in spec.events {
        let groups = hooks.entry(event.name).or_insert_with(|| json!([]));
        let groups = groups
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!(INVALID_CONFIG))?;
        let owned = group(spec, event);
        // Keep one current managed group per event; drop stale managed
        // handlers (legacy command forms) while preserving user handlers.
        groups.retain_mut(|existing| {
            if existing == &owned {
                return true;
            }
            if let Some(handlers) = existing.get_mut(HOOKS).and_then(Value::as_array_mut) {
                handlers.retain(|handler| !managed_handler(spec, handler));
            }
            existing
                .get(HOOKS)
                .and_then(Value::as_array)
                .is_some_and(|handlers| !handlers.is_empty())
        });
        if !groups.contains(&owned) {
            groups.push(owned);
        }
    }
    Ok(result)
}

pub(crate) fn restricted(spec: &HookInstallSpec, document: &Value) -> bool {
    spec.restricted_keys
        .iter()
        .any(|key| document.get(key).is_some_and(|value| value != false))
        || spec
            .disabled_paths
            .iter()
            .any(|path| disabled_at(document, path))
}

fn disabled_at(document: &Value, path: &[&str]) -> bool {
    let mut current = document;
    for key in path {
        let Some(next) = current.get(*key) else {
            return false;
        };
        current = next;
    }
    current.as_bool() == Some(false)
}

fn group(spec: &HookInstallSpec, event: &HookEventSpec) -> Value {
    let mut handler = json!({TYPE: COMMAND, COMMAND: callback_command(spec)});
    if let Some(timeout) = event.timeout_seconds {
        handler[TIMEOUT] = timeout.into();
    }
    json!({MATCHER: "", HOOKS: [handler]})
}

pub(crate) fn validate(document: &Value) -> Result<()> {
    let root = document
        .as_object()
        .ok_or_else(|| anyhow::anyhow!(INVALID_CONFIG))?;
    let Some(events) = root.get(HOOKS) else {
        return Ok(());
    };
    for groups in events
        .as_object()
        .ok_or_else(|| anyhow::anyhow!(INVALID_CONFIG))?
        .values()
    {
        for group in groups
            .as_array()
            .ok_or_else(|| anyhow::anyhow!(INVALID_CONFIG))?
        {
            let handlers = group
                .as_object()
                .and_then(|group| group.get(HOOKS))
                .and_then(Value::as_array)
                .ok_or_else(|| anyhow::anyhow!(INVALID_CONFIG))?;
            ensure!(handlers.iter().all(Value::is_object), INVALID_CONFIG);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../unit_tests/configuration.rs"]
mod tests;

/// Whether this provider already has at least one managed callback in the
/// document, in any historical command form.
pub fn contains_managed(spec: &HookInstallSpec, document: &Value) -> bool {
    document
        .get(HOOKS)
        .and_then(Value::as_object)
        .is_some_and(|events| {
            events.values().any(|groups| {
                groups.as_array().is_some_and(|groups| {
                    groups.iter().any(|group| {
                        group
                            .get(HOOKS)
                            .and_then(Value::as_array)
                            .is_some_and(|handlers| {
                                handlers
                                    .iter()
                                    .any(|handler| managed_handler(spec, handler))
                            })
                    })
                })
            })
        })
}

fn managed_handler(spec: &HookInstallSpec, handler: &Value) -> bool {
    handler
        .get(COMMAND)
        .and_then(Value::as_str)
        .is_some_and(|command| managed_command(spec, command))
}

/// A callback is ours when it invokes the managed Host hook executable with
/// the current or a legacy command form for this provider.
pub(crate) fn managed_command(spec: &HookInstallSpec, command: &str) -> bool {
    command.contains(yolo_agent_runtime::NATIVE_HOOK_EXECUTABLE_ENV)
        && command.contains(yolo_agent_runtime::NATIVE_HOOK_HOME_ENV)
        && spec
            .managed_hook_commands
            .iter()
            .any(|marker| command.contains(marker))
}
