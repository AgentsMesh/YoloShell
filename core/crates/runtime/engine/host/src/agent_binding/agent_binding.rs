pub(crate) const AGENT_RECOVERY_UNAVAILABLE_DETAIL: &str =
    "the detected agent launch has no durable recovery authority";
pub(crate) const RECREATE_REQUIRED_DETAIL: &str =
    "a plain shell has no provider history; explicit recreation is required";
pub(crate) const EXPLICIT_PROVIDER_RESUME_REQUIRED_DETAIL: &str =
    "an exited agent requires explicit provider resume";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResumeSelection {
    Preserve(ResumeIntent),
    ExplicitRecreate,
}

impl ResumeSelection {
    pub(crate) const fn explicit(recreate: bool) -> Self {
        if recreate {
            Self::ExplicitRecreate
        } else {
            Self::Preserve(ResumeIntent::Explicit)
        }
    }

    pub(crate) const fn intent(self) -> ResumeIntent {
        match self {
            Self::Preserve(intent) => intent,
            Self::ExplicitRecreate => ResumeIntent::Explicit,
        }
    }
}

#[derive(Debug)]
pub(crate) enum ResumeBinding {
    Provider(AgentBinding),
    Shell(ResumeBindingDisposition),
}

impl ResumeBinding {
    pub(crate) fn launch_command(&self, manifest: &TerminalManifest) -> ShellCommand {
        let cwd = match self {
            Self::Provider(binding) => Path::new(&binding.cwd),
            Self::Shell(_) => Path::new(&manifest.cwd),
        };
        shell_command(cwd)
    }
}


include!("agent_binding_load.rs");
include!("agent_binding_resume.rs");
include!("agent_binding_view.rs");
