use yolo_agent_baseline::Baseline;
use yolo_conversation_protocol::{
    ConversationDescriptor, ConversationEntry, ConversationPage, ConversationPart, ConversationRole,
};

pub(crate) fn absorb(
    baseline: &mut Baseline,
    new_entries: &mut Vec<ConversationEntry>,
    page: ConversationPage,
) {
    for entry in &page.entries {
        if !baseline.ids.contains(&entry.id) {
            baseline.ids.push(entry.id);
            remember_native(baseline, entry);
            new_entries.push(entry.clone());
        } else {
            remember_native(baseline, entry);
        }
    }
    baseline.generation = Some(page.history_generation);
    baseline.revision = Some(page.revision);
}

fn remember_native(baseline: &mut Baseline, entry: &ConversationEntry) {
    if let Some(native) = &entry.native_id
        && !baseline.native_ids.contains(native)
    {
        baseline.native_ids.push(native.clone());
    }
}

/// A rewritten history changes every entry id. Native record ids survive that
/// rewrite, so only records absent from the previous baseline are new.
pub(crate) fn rebase(
    baseline: &mut Baseline,
    new_entries: &mut Vec<ConversationEntry>,
    descriptor: ConversationDescriptor,
    page: ConversationPage,
) {
    new_entries.extend(new_native_entries(&baseline.native_ids, &page.entries));
    *baseline = Baseline::page(descriptor, page);
}

pub(crate) fn is_assistant_text(entry: &ConversationEntry) -> bool {
    entry.role == ConversationRole::Assistant
        && entry
            .parts
            .iter()
            .any(|part| matches!(part, ConversationPart::Text { .. }))
}

pub(crate) fn new_native_entries(
    native_ids: &[String],
    entries: &[ConversationEntry],
) -> Vec<ConversationEntry> {
    entries
        .iter()
        .filter(|entry| {
            entry
                .native_id
                .as_ref()
                .is_some_and(|native| !native_ids.contains(native))
        })
        .cloned()
        .collect()
}
