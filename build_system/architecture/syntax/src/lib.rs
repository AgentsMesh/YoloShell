pub const RUST_CFG_TEST_PREDICATE: &str = "test";

pub mod cargo_starlark_tokens;
pub mod rust_source;
pub mod rust_source_cfg;
pub mod rust_string_escape;
pub mod starlark_baseline;
pub mod starlark_document;
pub mod starlark_lists;
mod starlark_scan;
pub mod targeted_expression;
pub mod targeted_tokens;

#[cfg(test)]
#[path = "../unit_tests/rust_source_cfg_tests.rs"]
mod rust_source_cfg_tests;
#[cfg(test)]
#[path = "../unit_tests/rust_string_escape_tests.rs"]
mod rust_string_escape_tests;
#[cfg(test)]
#[path = "../unit_tests/starlark_baseline_tests.rs"]
mod starlark_baseline_tests;
#[cfg(test)]
#[path = "../unit_tests/starlark_lists_tests.rs"]
mod starlark_lists_tests;
#[cfg(test)]
#[path = "../unit_tests/starlark_document_boundaries.rs"]
mod starlark_document_boundaries;
#[cfg(test)]
#[path = "../unit_tests/targeted_expression_tests.rs"]
mod targeted_expression_tests;
#[cfg(test)]
#[path = "../unit_tests/targeted_token_lexer_tests.rs"]
mod targeted_token_lexer_tests;
