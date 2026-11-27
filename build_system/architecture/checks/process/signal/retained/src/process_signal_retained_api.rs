#[path = "process_signal_retained_api/associated.rs"]
mod associated;

use yolo_architecture_foundation::SourceFile;
use yolo_architecture_foundation::model::Violation;
use yolo_architecture_policy_contracts::process_signal as contract;
use yolo_architecture_policy_contracts::process_signal_retained as retained;
use yolo_architecture_process_signal_foundation::process_signal_guardian_constants as guardian;
use yolo_architecture_process_signal_path_analysis::visibility_fields as visibility;
use yolo_architecture_syntax::rust_source::{RustFunction, RustSource};
use yolo_architecture_syntax::targeted_tokens::{Token, closing, tokenize};

const BLOCK_OPEN: &str = "{";
const BLOCK_CLOSE: &str = "}";
pub const EXACT_TEST_SUPPORT_PATH: &str =
    "core/crates/runtime/engine/process/test_support/exact_termination.rs";

pub(crate) use associated::{Surface as AssociatedSurface, inspect as inspect_associated_types};

#[rustfmt::skip]
const EXACT_METHODS: &[(&str, &str)] = &[
    (contract::CAPTURE, "pub fn capture(identity: ProcessIdentity) -> Result<Option<Self>> {"),
    (retained::CAPTURE_PTY_FUNCTION, "pub(crate) fn capture_pty(identity: ProcessIdentity, proof: super::PtySessionProof,) -> Result<Option<Self>> {"),
    (retained::CAPTURE_PTY_FOR_TEST_FUNCTION, "#[cfg(feature = \"adversarial-test-hooks\")] pub fn capture_pty_for_test(identity: ProcessIdentity, proof: super::PtySessionProof,) -> Result<Option<Self>> {"),
    (retained::CAPTURE_PTY_EXCLUDING_FUNCTION, "pub fn capture_pty_excluding(identity: ProcessIdentity, proof: super::PtySessionProof, excluded: impl IntoIterator<Item = ProcessIdentity>,) -> Result<Option<Self>> {"),
    (retained::CAPTURE_WITH_FUNCTION, "fn capture_with(identity: ProcessIdentity, capture: impl FnOnce(ProcessIdentity) -> Result<CapturedProcessTree>,) -> Result<Option<CapturedProcessTree>> {"),
    (guardian::OBSERVE_OPERATION, "pub fn observe(&mut self) -> Result<RetainedProcessObservation> {"),
    (contract::IS_RUNNING, "pub fn is_running(&mut self) -> Result<bool> {"),
    (contract::BEGIN_CONTAINMENT, "pub fn begin_containment(&mut self) -> Result<bool> {"),
    (contract::ENFORCE_CONTAINMENT, "pub fn enforce_containment(&mut self) -> Result<bool> {"),
    (retained::DESTRUCTIVE_FUNCTION, "fn destructive(tree: CapturedProcessTree) -> Self {"),
    (retained::EXACT_OBSERVER_FUNCTION, "fn exact_observer(tree: CapturedProcessTree) -> Self {"),
    (retained::TREE_FUNCTION, "fn tree(&self) -> &CapturedProcessTree {"),
    (retained::TREE_MUT_FUNCTION, "#[cfg(any(test, feature = \"adversarial-test-hooks\"))] fn tree_mut(&mut self) -> &mut CapturedProcessTree {"),
];
#[rustfmt::skip]
const EXACT_TEST_METHODS: &[(&str, &str)] = &[
    (retained::FAIL_NEXT_REFRESH_FUNCTION, "pub fn fail_next_refresh_for_test(&mut self, detail: &str) {"),
    (retained::IS_EXACT_OBSERVER_FOR_TEST_FUNCTION, "#[cfg(any(test, feature = \"runtime-component-tests\"))] pub fn is_exact_observer_for_test(&self) -> bool {"),
    (retained::STATE_SNAPSHOT_FOR_TEST_FUNCTION, "#[cfg(any(test, feature = \"runtime-component-tests\"))] pub fn state_snapshot_for_test(&self) -> crate::CapturedProcessTreeStateSnapshot {"),
];
pub fn inspect(file: &SourceFile, source: &RustSource, violations: &mut Vec<Violation>) {
    for function in source
        .functions()
        .filter(|function| function.owner.as_deref() == Some(contract::RETAINED_EXACT_PROCESS))
    {
        let methods = match file.relative.as_str() {
            contract::EXACT_TERMINATION_PATH => EXACT_METHODS,
            EXACT_TEST_SUPPORT_PATH => EXACT_TEST_METHODS,
            _ => &[],
        };
        let allowed = methods.iter().any(|(name, header)| {
            function.name == *name && header_matches(source, function, header)
        });
        if !allowed {
            violations.push(Violation::line(
                &file.relative,
                function.line,
                retained::UNCLASSIFIED_METHOD_MESSAGE,
            ));
        }
    }
}

pub fn allows_surface(_path: &str, _source: &RustSource, _function: &RustFunction) -> bool {
    false
}

pub fn block(tokens: &[Token], start: usize) -> Option<(usize, usize)> {
    let open = tokens[start..]
        .iter()
        .position(|token| token.symbol(BLOCK_OPEN))?
        + start;
    Some((open, closing(tokens, open, BLOCK_OPEN, BLOCK_CLOSE)?))
}

fn header_matches(source: &RustSource, function: &RustFunction, expected: &str) -> bool {
    let Some(range) = visibility::function_header(&source.tokens, function.body.start) else {
        return false;
    };
    tokens_match(&source.tokens[range], expected)
}

pub(super) fn tokens_match(tokens: &[Token], expected: &str) -> bool {
    let expected = tokenize(expected);
    tokens
        .iter()
        .map(|token| &token.kind)
        .eq(expected.iter().map(|token| &token.kind))
}
