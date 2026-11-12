use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal_retained as retained;
use yolo_architecture_syntax::rust_source::RustSource;
use yolo_architecture_syntax::targeted_tokens::Kind;

pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    let unsupported = source
        .tokens
        .iter()
        .find(|token| matches!(&token.kind, Kind::Symbol(value) if !value.is_ascii()));
    if let Some(token) = unsupported {
        violations.push(Violation::line(
            &file.relative,
            token.line,
            retained::UNSUPPORTED_IDENTIFIER_MESSAGE,
        ));
    }
}
