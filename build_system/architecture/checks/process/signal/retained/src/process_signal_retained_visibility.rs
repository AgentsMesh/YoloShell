#[path = "process_signal_retained_visibility/containment.rs"]
mod containment;
#[path = "process_signal_retained_visibility/surfaces.rs"]
mod surfaces;

use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_process_signal_foundation::process_signal_retained_export_sites as sites;
use yolo_architecture_build_graph::modules::ModuleIndex;
use yolo_architecture_process_signal_path_analysis::types::RetainedTypes;
use yolo_architecture_process_signal_path_analysis::visibility_fields as syntax;
use yolo_architecture_syntax::rust_source::{RustSource, identifier};
use yolo_architecture_syntax::targeted_tokens::Token;

const TYPE_KEYWORD: &str = "type";
const TYPE_ITEMS: &[&str] = &["struct", "enum", "union", TYPE_KEYWORD];
const NEXT_TOKEN_OFFSET: usize = 1;

pub fn declared_types(source: &RustSource, retained: &RetainedTypes) -> Vec<(usize, String, bool)> {
    declarations(&source.tokens)
        .into_iter()
        .filter_map(|(index, name)| {
            retained
                .declaration_canonical(index)
                .map(|canonical| (index, name.to_owned(), canonical))
        })
        .collect()
}

pub fn inspect_with_modules(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    modules: &ModuleIndex,
    violations: &mut Vec<Violation>,
) {
    let context = containment::Context::new(&file.relative, &source.tokens, retained, modules);
    inspect_declarations(file, source, retained, &context, violations);
    surfaces::inspect(file, source, retained, Some(&context), violations);
}

fn inspect_declarations(
    file: &SourceFile,
    source: &RustSource,
    retained: &RetainedTypes,
    context: &containment::Context<'_>,
    violations: &mut Vec<Violation>,
) {
    for (index, name) in declarations(&source.tokens) {
        if retained.declaration_canonical(index).is_some()
            && context.module_item(index)
            && syntax::item_visible(&source.tokens, index)
            && !sites::allows_type(&file.relative, name)
            && !context.allows_declaration(index)
        {
            syntax::violation(file, &source.tokens[index], violations);
        }
    }
}

fn declarations(tokens: &[Token]) -> Vec<(usize, &str)> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| type_item(token))
        .filter_map(|(index, _)| {
            tokens
                .get(index + NEXT_TOKEN_OFFSET)
                .and_then(identifier)
                .map(|name| (index, name))
        })
        .collect()
}

fn type_item(token: &Token) -> bool {
    TYPE_ITEMS.iter().any(|keyword| token.identifier(keyword))
}
