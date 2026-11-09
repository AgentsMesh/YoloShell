use anyhow::Result;

pub fn error_text<T>(result: Result<T>) -> String {
    format!("{:#}", result.err().expect("expected error"))
}
