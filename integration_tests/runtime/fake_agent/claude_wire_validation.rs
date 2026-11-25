use super::*;
use uuid::Uuid;

impl ClaudeHookFrame {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != WIRE_SCHEMA_VERSION {
            return Err(WIRE_SCHEMA_UNSUPPORTED);
        }
        self.authority.validate()?;
        if !canonical_uuid(&self.event_id) || !self.input.valid() {
            return Err(WIRE_EVENT_INVALID);
        }
        Ok(())
    }
}

impl HookAuthority {
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        let ids = [
            self.terminal_id.as_str(),
            self.runtime_epoch.as_str(),
            self.invocation_id.as_str(),
            self.hook_token.as_str(),
        ];
        let identities = [self.worker, self.shell, self.agent, self.hook];
        let lineage = self.shell.parent_pid == Some(self.worker.pid)
            && self.agent.parent_pid == Some(self.shell.pid)
            && self.hook.parent_pid == Some(self.agent.pid);
        let distinct = identities.iter().enumerate().all(|(index, identity)| {
            identities[..index]
                .iter()
                .all(|seen| seen.pid != identity.pid)
        });
        if ids.into_iter().all(canonical_uuid)
            && identities.iter().all(ProcessIdentity::valid)
            && lineage
            && distinct
        {
            Ok(())
        } else {
            Err(WIRE_AUTHORITY_INVALID)
        }
    }
}

impl ProcessIdentity {
    fn valid(&self) -> bool {
        self.pid != 0 && self.start_time != 0
    }
}

impl ClaudeHookInput {
    pub(crate) fn common(&self) -> &CommonHookInput {
        match self {
            Self::SessionStart { common, .. }
            | Self::SessionEnd { common, .. }
            | Self::CwdChanged { common, .. }
            | Self::PreCompact { common, .. }
            | Self::PostCompact { common, .. }
            | Self::UserPromptSubmit { common, .. }
            | Self::Stop { common, .. } => common,
        }
    }

    fn valid(&self) -> bool {
        let common = self.common();
        let path_valid = common.cwd.is_absolute()
            && common.transcript_path.is_absolute()
            && common
                .transcript_path
                .file_stem()
                .and_then(|value| value.to_str())
                == Some(common.session_id.as_str());
        let cwd_valid = match self {
            Self::CwdChanged {
                common, new_cwd, ..
            } => common.cwd == *new_cwd,
            _ => true,
        };
        canonical_uuid(&common.session_id) && path_valid && cwd_valid
    }
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value)
        .ok()
        .is_some_and(|parsed| parsed.hyphenated().to_string() == value)
}
