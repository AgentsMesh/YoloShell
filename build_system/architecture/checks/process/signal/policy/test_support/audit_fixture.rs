use yolo_architecture_foundation::SourceFile;
use yolo_architecture_syntax::rust_source::RustSource;

pub const OTHER_PATH: &str = "core/src/other.rs";
pub const TEST_PATH: &str = "core/unit_tests/other.rs";
pub const PROBE: &str = "probe";

pub fn check(
    path: &str,
    text: &str,
    needle: &str,
    predicate: fn(&SourceFile, &RustSource, usize) -> bool,
) -> bool {
    let source = RustSource::parse(text);
    let index = source
        .tokens
        .iter()
        .rposition(|token| token.identifier(needle))
        .unwrap();
    let file = SourceFile {
        absolute: path.into(),
        relative: path.into(),
        text: text.into(),
    };
    predicate(&file, &source, index)
}
