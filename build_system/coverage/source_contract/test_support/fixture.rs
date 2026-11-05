use crate::constants::TEST_SOURCE;

pub fn manifest(sources: &[(&str, &str)]) -> String {
    sources
        .iter()
        .map(|(source, _)| *source)
        .chain([TEST_SOURCE])
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn read(source: &str) -> Result<Vec<u8>, String> {
    std::fs::read(source).map_err(|error| error.to_string())
}
