use crate::constants::{EXIT_REJECTED, EXIT_SUCCESS, EXIT_TIMEOUT, EXIT_UNKNOWN};
use anyhow::Result;
use serde_json::{Value, json};
use std::io::Write;

pub struct Outcome {
    pub code: i32,
    pub json: bool,
    pub value: Value,
    pub text: String,
}

impl Outcome {
    pub fn success(json_mode: bool, value: Value, text: String) -> Self {
        Self {
            code: EXIT_SUCCESS,
            json: json_mode,
            value,
            text,
        }
    }

    pub fn failure(code: i32, json_mode: bool, message: String) -> Self {
        let label = match code {
            EXIT_TIMEOUT => "timeout",
            EXIT_UNKNOWN => "unknown",
            _ => "rejected",
        };
        Self {
            code,
            json: json_mode,
            value: json!({ "error": { "code": label, "message": message } }),
            text: message,
        }
    }

    pub fn print(&self) -> Result<()> {
        let stdout = std::io::stdout();
        let mut output = stdout.lock();
        if self.json {
            serde_json::to_writer_pretty(&mut output, &self.value)?;
            writeln!(output)?;
        } else if !self.text.is_empty() {
            writeln!(output, "{}", self.text)?;
        }
        output.flush()?;
        Ok(())
    }
}

pub fn reject(json_mode: bool, error: anyhow::Error) -> Result<Outcome> {
    match error.downcast::<crate::reject::Reject>() {
        Ok(reject) => Ok(Outcome::failure(
            EXIT_REJECTED,
            json_mode,
            reject.to_string(),
        )),
        Err(error) => Err(error),
    }
}
