use crate::retired_wire_constants::*;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::targeted_tokens::{Kind, tokenize};

pub(super) fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !file.relative.starts_with(CORE_ROOT)
        || !file.relative.contains(RUST_SOURCE)
        || !file.relative.ends_with(RUST_EXTENSION)
    {
        return;
    }
    for token in tokenize(&file.text) {
        let retired = match &token.kind {
            Kind::Identifier(value) => RETIRED_IDENTIFIERS.contains(&value.as_str()),
            Kind::String(value) => RETIRED_WIRE_VALUES.contains(&value.as_str()),
            Kind::Number(_) | Kind::Symbol(_) => false,
        };
        if retired {
            violations.push(Violation::line(&file.relative, token.line, MESSAGE));
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/retired_wire.rs"]
mod tests;
