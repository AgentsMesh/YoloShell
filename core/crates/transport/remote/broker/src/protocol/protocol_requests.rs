use super::constants::{DEFAULT_COMMAND_OUTPUT_BYTES, DEFAULT_COMMAND_TIMEOUT_MILLIS};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerCommandRequest {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub timeout_millis: u64,
    pub output_limit: usize,
}

impl BrokerCommandRequest {
    pub fn standard(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
            cwd: None,
            timeout_millis: DEFAULT_COMMAND_TIMEOUT_MILLIS,
            output_limit: DEFAULT_COMMAND_OUTPUT_BYTES,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BrokerFileRequest {
    pub path: PathBuf,
    pub offset: u64,
    pub length: u64,
}
