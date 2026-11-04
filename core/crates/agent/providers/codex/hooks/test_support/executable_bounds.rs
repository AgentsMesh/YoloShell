use std::path::Path;

#[path = "executable_bounds_constants.rs"]
mod constants;
pub(super) use constants::*;

pub(super) fn assert_invalid<T>(result: anyhow::Result<T>) {
    assert_eq!(
        result.err().expect(INVALID_EXECUTABLE).to_string(),
        INVALID_EXECUTABLE
    );
}

pub(super) fn resize(path: &Path, length: u64) -> std::fs::File {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    file.set_len(length).unwrap();
    file
}
