use crate::build_entrypoint_constants::*;
use yolo_architecture_foundation::{SourceFile, model::Violation};
use yolo_architecture_syntax::starlark_document::Document;

pub(super) fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if RETIRED_MACRO_FILES.contains(&file.relative.as_str()) {
        violations.push(Violation::file(&file.relative, MESSAGE));
        return;
    }
    if !file.relative.ends_with(STARLARK_EXTENSION) && !file.relative.ends_with(BAZEL_EXTENSION) {
        return;
    }
    let document = Document::parse(&file.text);
    for (package, names) in [
        (CORE_PACKAGE, CORE_FORWARDERS),
        (INTEGRATION_PACKAGE, INTEGRATION_FORWARDERS),
    ] {
        let is_package = file.relative == format!("{package}/{BUILD_FILE}");
        let declared = is_package
            && document.all_calls().iter().any(|call| {
                document
                    .keyword_string(call, NAME_ATTRIBUTE)
                    .is_some_and(|name| names.contains(&name))
            });
        let referenced = names.iter().any(|name| {
            document.string_count(&format!("//{package}:{name}")) > 0
                || is_package && document.string_count(&format!(":{name}")) > 0
        });
        if declared || referenced {
            violations.push(Violation::file(&file.relative, MESSAGE));
        }
    }
}

#[cfg(test)]
#[path = "../unit_tests/build_entrypoint.rs"]
mod tests;
