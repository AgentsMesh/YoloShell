use crate::{CWD_ARGUMENT, DATA_HOME_ENV, KIND, RESUME_ARGUMENT};
pub use crate::{LEGACY_SETTING, NO_DAEMON};
use std::sync::LazyLock;
use yolo_agent_contract::{AgentBinding, ResumePlan};

const SHELL: &str = "/bin/sh";
// Both the feature probe and the invocation use the PATH of the new terminal.
// The shell is replaced only after choosing the arguments, so an exited Codex
// never starts a second copy.
const SCRIPT: &str = r#"if command "$0" --no-daemon --version </dev/null >/dev/null 2>&1; then exec "$0" --no-daemon "$@"; else exec "$0" --config features.daemon_auto_start=false "$@"; fi"#;

pub fn start_command() -> &'static str {
    static COMMAND: LazyLock<String> = LazyLock::new(|| format!("{SHELL} -c '{SCRIPT}' codex"));
    &COMMAND
}

pub fn plain_resume(binding: &AgentBinding) -> ResumePlan {
    ResumePlan {
        executable: KIND.into(),
        args: vec![
            RESUME_ARGUMENT.into(),
            CWD_ARGUMENT.into(),
            binding.cwd.clone(),
            binding.provider_session_id.clone(),
        ],
        environment: (!binding.data_home.is_empty())
            .then(|| (DATA_HOME_ENV.into(), binding.data_home.clone()))
            .into_iter()
            .collect(),
    }
}

pub fn resume(mut plan: ResumePlan) -> ResumePlan {
    let executable = std::mem::replace(&mut plan.executable, SHELL.into());
    let mut args = vec!["-c".into(), SCRIPT.into(), executable];
    args.append(&mut plan.args);
    plan.args = args;
    plan
}
