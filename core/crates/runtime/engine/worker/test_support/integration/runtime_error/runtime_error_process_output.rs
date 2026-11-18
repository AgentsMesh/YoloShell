const RUNTIME_ERROR_PROCESS_DIAGNOSTIC_KIB: u64 = 8;
const RUNTIME_ERROR_PROCESS_BYTES_PER_KIB: u64 = 1024;
const RUNTIME_ERROR_PROCESS_DIAGNOSTIC_LIMIT: u64 =
    RUNTIME_ERROR_PROCESS_DIAGNOSTIC_KIB * RUNTIME_ERROR_PROCESS_BYTES_PER_KIB;
const RUNTIME_ERROR_PROCESS_STDOUT_CREATE_DETAIL: &str =
    "create concurrent runtime-error stdout capture";
const RUNTIME_ERROR_PROCESS_STDERR_CREATE_DETAIL: &str =
    "create concurrent runtime-error stderr capture";
const RUNTIME_ERROR_PROCESS_OUTPUT_CLONE_DETAIL: &str =
    "clone concurrent runtime-error output capture";
const RUNTIME_ERROR_PROCESS_OUTPUT_READ_FAILURE: &str =
    "concurrent runtime-error output could not be read";
const RUNTIME_ERROR_PROCESS_STDOUT_LABEL: &str = "stdout";
const RUNTIME_ERROR_PROCESS_STDERR_LABEL: &str = "stderr";

struct RuntimeErrorProcessOutput {
    stdout: std::fs::File,
    stderr: std::fs::File,
}

impl RuntimeErrorProcessOutput {
    fn capture(command: &mut std::process::Command) -> Result<Self> {
        let stdout = tempfile::tempfile().context(RUNTIME_ERROR_PROCESS_STDOUT_CREATE_DETAIL)?;
        let stderr = tempfile::tempfile().context(RUNTIME_ERROR_PROCESS_STDERR_CREATE_DETAIL)?;
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(
                stdout
                    .try_clone()
                    .context(RUNTIME_ERROR_PROCESS_OUTPUT_CLONE_DETAIL)?,
            ))
            .stderr(std::process::Stdio::from(
                stderr
                    .try_clone()
                    .context(RUNTIME_ERROR_PROCESS_OUTPUT_CLONE_DETAIL)?,
            ));
        Ok(Self { stdout, stderr })
    }

    fn diagnostics(&mut self) -> String {
        let stdout = read_runtime_error_process_output(&mut self.stdout);
        let stderr = read_runtime_error_process_output(&mut self.stderr);
        format!(
            "{RUNTIME_ERROR_PROCESS_STDOUT_LABEL}={stdout:?}; \
             {RUNTIME_ERROR_PROCESS_STDERR_LABEL}={stderr:?}"
        )
    }
}

fn read_runtime_error_process_output(output: &mut std::fs::File) -> String {
    if std::io::Seek::rewind(output).is_err() {
        return String::from(RUNTIME_ERROR_PROCESS_OUTPUT_READ_FAILURE);
    }
    let mut bytes = Vec::new();
    let mut bounded = std::io::Read::take(output, RUNTIME_ERROR_PROCESS_DIAGNOSTIC_LIMIT);
    if std::io::Read::read_to_end(&mut bounded, &mut bytes).is_err() {
        return String::from(RUNTIME_ERROR_PROCESS_OUTPUT_READ_FAILURE);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}
