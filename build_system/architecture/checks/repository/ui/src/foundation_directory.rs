use yolo_architecture_foundation::SourceFile;
use yolo_architecture_syntax::targeted_tokens::tokenize;

const ADAPTER: &str = "core/crates/application/local/profile/src/foundation.rs";
const RUNTIME: &str = "objc";
const APPROVED_BINDING: &str = include_str!("foundation_directory.fixture");

/// One platform adapter may resolve the same filesystem directory as Foundation.
/// This does not permit a general Objective-C bridge or any UI framework in core.
pub(super) fn permits(file: &SourceFile, term: &str) -> bool {
    if file.relative != ADAPTER || term != RUNTIME {
        return false;
    }
    // Keep the reviewed module independent of the live adapter. Token order and
    // multiplicity are significant; only comments, whitespace, and lines vary.
    tokenize(&file.text)
        .into_iter()
        .map(|token| token.kind)
        .eq(tokenize(APPROVED_BINDING)
            .into_iter()
            .map(|token| token.kind))
}
