use std::path::Path;

use crate::source_constants::{
    RUST_EXTENSION, TEST_DIRECTORY_STEMS, TEST_DIRECTORY_SUFFIXES, TEST_FILE_STEMS,
    TEST_FILE_SUFFIXES,
};

/// The single production/test classification for Rust source paths.
pub fn is_rust_test_source(relative: &str) -> bool {
    let path = Path::new(relative);
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return false;
    };
    path.extension().and_then(|value| value.to_str()) == Some(RUST_EXTENSION)
        && (test_directory(path)
            || TEST_FILE_STEMS.contains(&stem)
            || TEST_FILE_SUFFIXES
                .iter()
                .any(|suffix| stem.ends_with(suffix)))
}

pub fn is_production_rust_source(relative: &str) -> bool {
    relative.ends_with(".rs") && !is_rust_test_source(relative)
}

fn test_directory(path: &Path) -> bool {
    path.parent().is_some_and(|parent| {
        parent.components().any(|component| {
            component.as_os_str().to_str().is_some_and(|name| {
                TEST_DIRECTORY_STEMS.contains(&name)
                    || TEST_DIRECTORY_SUFFIXES
                        .iter()
                        .any(|suffix| name.ends_with(suffix))
            })
        })
    })
}
