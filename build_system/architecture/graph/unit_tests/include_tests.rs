use super::*;
use yolo_architecture_syntax::rust_source::RustSource;

#[test]
fn include_test_guard_is_preserved_on_the_edge() {
    let source = RustSource::parse("#[cfg(test)] include!(\"tests.rs\");");
    let invocation = parse(&source.tokens).pop().expect("include invocation");
    assert!(invocation.test_only);
}
