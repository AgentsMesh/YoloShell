use super::RemoteError;
use super::constants::{MAX_COMMAND_ARGUMENTS, MAX_COMMAND_OUTPUT_BYTES, MAX_COMMAND_TIMEOUT};
use super::file::scoped_absolute;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    program: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    timeout: Duration,
    output_limit: usize,
}

impl CommandRequest {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
            cwd: None,
            timeout: MAX_COMMAND_TIMEOUT,
            output_limit: MAX_COMMAND_OUTPUT_BYTES,
        }
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn with_limits(mut self, timeout: Duration, output_limit: usize) -> Self {
        self.timeout = timeout;
        self.output_limit = output_limit;
        self
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }

    pub fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn output_limit(&self) -> usize {
        self.output_limit
    }
}

#[derive(Debug, Clone)]
pub struct CommandPolicy {
    allowed_programs: BTreeSet<String>,
}

impl CommandPolicy {
    pub fn new(programs: impl IntoIterator<Item = String>) -> Result<Self, RemoteError> {
        let allowed_programs = programs.into_iter().collect::<BTreeSet<_>>();
        if allowed_programs.is_empty() || allowed_programs.iter().any(|value| !safe_program(value))
        {
            return Err(RemoteError::InvalidCommand(
                "allowlist must contain safe program names".into(),
            ));
        }
        Ok(Self { allowed_programs })
    }

    pub fn authorize(&self, request: CommandRequest) -> Result<AuthorizedCommand, RemoteError> {
        validate_request(&request)?;
        if !self.allowed_programs.contains(request.program()) {
            return Err(RemoteError::CommandDenied(request.program().into()));
        }
        Ok(AuthorizedCommand(request))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedCommand(pub(crate) CommandRequest);

impl AuthorizedCommand {
    pub fn request(&self) -> &CommandRequest {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_status: Option<u32>,
    pub timed_out: bool,
    pub truncated: bool,
}

impl CommandOutput {
    pub fn validate(&self, command: &AuthorizedCommand) -> Result<(), RemoteError> {
        let total = self.stdout.len().saturating_add(self.stderr.len());
        if total > command.request().output_limit() {
            return Err(RemoteError::LimitExceeded);
        }
        Ok(())
    }
}

fn validate_request(request: &CommandRequest) -> Result<(), RemoteError> {
    if !safe_program(request.program())
        || request.args().len() > MAX_COMMAND_ARGUMENTS
        || request.args().iter().any(|value| unsafe_text(value))
        || request.cwd().is_some_and(|path| !scoped_absolute(path))
        || request.timeout().is_zero()
        || request.timeout() > MAX_COMMAND_TIMEOUT
        || request.output_limit() == 0
        || request.output_limit() > MAX_COMMAND_OUTPUT_BYTES
    {
        return Err(RemoteError::InvalidCommand(
            "arguments, cwd, timeout, or output limit is invalid".into(),
        ));
    }
    Ok(())
}

fn unsafe_text(value: &str) -> bool {
    value.is_empty() || value.contains('\0')
}

fn safe_program(value: &str) -> bool {
    let path = Path::new(value);
    !unsafe_text(value) && scoped_absolute(path) && path.file_name().is_some()
}
