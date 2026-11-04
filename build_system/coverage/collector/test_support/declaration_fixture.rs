use crate::coverage_json::Data;
use crate::instance_fixture::{self, FILE_INDEX, TEST_SOURCE};
use std::path::Path;

pub fn data() -> Data {
    data_for(TEST_SOURCE)
}

pub fn data_for(source: &str) -> Data {
    let mut function = instance_fixture::function();
    function.filenames[FILE_INDEX as usize] = source.to_owned();
    let mut data = instance_fixture::data(vec![function]);
    data.files[FILE_INDEX as usize].filename = source.to_owned();
    data
}

pub fn manifest(sources: &[(&str, &str)]) -> String {
    sources
        .iter()
        .map(|(source, _)| *source)
        .chain([TEST_SOURCE])
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn read(source: &str) -> Result<Vec<u8>, String> {
    crate::declaration_sources::read_bounded(Path::new(source))
}
