use std::path::PathBuf;

#[derive(Debug)]
pub struct SourceFile {
    pub absolute: PathBuf,
    pub relative: String,
    pub text: String,
}
