use super::{Context, records};
use yolo_architecture_process_signal_foundation::process_signal_retained_export_sites as sites;
use yolo_architecture_process_signal_path_analysis::visibility_fields as syntax;

const ENUM_KEYWORD: &str = "enum";

pub(super) fn allowed(context: &Context<'_>, index: usize) -> bool {
    if !context.certain || sites::requires_exact_type_path(context.relative) {
        return false;
    }
    context
        .aggregates
        .iter()
        .find(|aggregate| aggregate.declaration == index)
        .is_some_and(|aggregate| body_allowed(context, aggregate))
}

fn body_allowed(context: &Context<'_>, aggregate: &records::Aggregate) -> bool {
    if !context.contains_reference(aggregate.body.clone()) {
        return false;
    }
    if context.tokens[aggregate.declaration].identifier(ENUM_KEYWORD) {
        let references = context
            .references(aggregate.body.clone())
            .collect::<Vec<_>>();
        return references.iter().all(|index| {
            match context.aggregate_exposure(*index, &aggregate.visible_from) {
                Some(allowed) => allowed,
                None => context.retained.contains_canonical(context.tokens, *index),
            }
        });
    }
    syntax::fields(context.tokens, aggregate.declaration).is_some()
}
