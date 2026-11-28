use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_source_policy::source_policy::is_rust_product_source;
use yolo_architecture_syntax::rust_source::{RustSource, string};
use yolo_architecture_syntax::targeted_tokens::Kind;

const THAW_NAME: &str = "thaw";
const CGROUP_THAW_VALUE: &str = "0\n";
const POLICY_MESSAGE: &str =
    "production Linux containment must never expose or write cgroup thaw authority";

pub fn inspect(file: &SourceFile, violations: &mut Vec<Violation>) {
    if !production_rust(file) {
        return;
    }
    let source = RustSource::parse(&file.text);
    for (index, token) in source.tokens.iter().enumerate() {
        let named_thaw = match &token.kind {
            Kind::Identifier(name) => name.to_ascii_lowercase().contains(THAW_NAME),
            _ => false,
        };
        let literal_thaw = string(token) == Some(CGROUP_THAW_VALUE);
        if (named_thaw || literal_thaw) && !test_guarded(&source, index) {
            violations.push(Violation::line(&file.relative, token.line, POLICY_MESSAGE));
        }
    }
}

fn production_rust(file: &SourceFile) -> bool {
    is_rust_product_source(&file.relative)
}

fn test_guarded(source: &RustSource, index: usize) -> bool {
    source
        .function_at(index)
        .is_some_and(|function| source.is_test_guarded(function))
}
