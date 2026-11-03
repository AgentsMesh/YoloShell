use crate::constants::MAX_RETAINED_LIFECYCLE_ENTRIES;
use crate::event::{EventKind, HookEnvelope, ToolSet};
use crate::state::NormalizedLifecycle;

pub(crate) fn is_within_limit(state: &NormalizedLifecycle) -> bool {
    retained_entries(state).is_some_and(|count| count <= MAX_RETAINED_LIFECYCLE_ENTRIES)
}

pub(crate) fn admits_transition(state: &NormalizedLifecycle, event: &HookEnvelope) -> bool {
    let Some(mut count) = retained_entries(state) else {
        return false;
    };
    if resets_lifecycle(event) {
        return true;
    }
    if matches!(&event.event, EventKind::SubagentStart) {
        return add(&mut count, true) && count <= MAX_RETAINED_LIFECYCLE_ENTRIES;
    }
    let projected = if let Some(agent) = event.agent_id.as_deref() {
        project_child(state, agent, event, &mut count)
    } else {
        project_tools(&state.active_tools, event, &mut count)
    };
    projected && count <= MAX_RETAINED_LIFECYCLE_ENTRIES
}

fn retained_entries(state: &NormalizedLifecycle) -> Option<usize> {
    let child_tools = state.children.values().try_fold(0_usize, |count, child| {
        count.checked_add(child.active_tools.len())
    })?;
    state
        .children
        .len()
        .checked_add(state.active_tools.len())?
        .checked_add(child_tools)?
        .checked_add(state.child_compactions.len())
}

fn resets_lifecycle(event: &HookEnvelope) -> bool {
    matches!(
        &event.event,
        EventKind::SessionStart {
            source: crate::event::SessionStartSource::Startup
                | crate::event::SessionStartSource::Resume
                | crate::event::SessionStartSource::Clear
                | crate::event::SessionStartSource::Fork
        }
    )
}

fn project_child(
    state: &NormalizedLifecycle,
    agent: &str,
    event: &HookEnvelope,
    count: &mut usize,
) -> bool {
    let Some(child) = state.children.get(agent) else {
        return false;
    };
    let compaction_exists = state.child_compactions.contains_key(agent);
    let compaction_after = match &event.event {
        EventKind::PreCompact => true,
        EventKind::Notification { .. } | EventKind::CwdChanged => compaction_exists,
        _ => false,
    };
    adjust(count, compaction_exists, compaction_after)
        && project_tools(&child.active_tools, event, count)
}

fn project_tools(tools: &ToolSet, event: &HookEnvelope, count: &mut usize) -> bool {
    let selected = event.tool_use_id.as_deref().or(event.tool_name.as_deref());
    match &event.event {
        EventKind::PreToolUse => add(count, selected.is_some_and(|tool| !tools.contains(tool))),
        EventKind::PostToolUse | EventKind::PostToolUseFailure => {
            remove(count, selected.is_some_and(|tool| tools.contains(tool)))
        }
        EventKind::SubagentStop
        | EventKind::Stop
        | EventKind::StopFailure
        | EventKind::SessionEnd { .. } => remove_many(count, tools.len()),
        _ => true,
    }
}

fn adjust(count: &mut usize, before: bool, after: bool) -> bool {
    match (before, after) {
        (false, true) => add(count, true),
        (true, false) => remove(count, true),
        _ => true,
    }
}

fn add(count: &mut usize, present: bool) -> bool {
    !present
        || count.checked_add(1).is_some_and(|next| {
            *count = next;
            true
        })
}

fn remove(count: &mut usize, present: bool) -> bool {
    !present
        || count.checked_sub(1).is_some_and(|next| {
            *count = next;
            true
        })
}

fn remove_many(count: &mut usize, removed: usize) -> bool {
    count.checked_sub(removed).is_some_and(|next| {
        *count = next;
        true
    })
}
