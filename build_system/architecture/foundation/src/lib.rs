pub const MIN_SEMANTIC_LITERAL_LENGTH: usize = 3;

pub mod model;
mod source_constants;
pub mod source_file;
pub mod source_path;
pub mod text;

pub use source_file::SourceFile;

#[cfg(test)]
#[path = "../unit_tests/source_path_tests.rs"]
mod source_path_tests;
#[cfg(test)]
#[path = "../unit_tests/text_tests.rs"]
mod text_tests;
