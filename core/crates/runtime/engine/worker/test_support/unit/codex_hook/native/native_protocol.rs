use super::{Fixture, IngressTestAssets};
use crate::codex_hooks::{CodexHookInput, CodexNativeRequest, NATIVE_SCHEMA_VERSION};

const MODEL: &str = "gpt-5";
const PERMISSION: &str = "default";
const SOURCE: &str = "startup";

pub(super) fn native_request(assets: &IngressTestAssets, fixture: &Fixture) -> CodexNativeRequest {
    let context = assets.request(fixture);
    CodexNativeRequest {
        schema_version: NATIVE_SCHEMA_VERSION,
        terminal_id: context.identity.terminal_id,
        runtime_epoch: context.identity.runtime_epoch,
        event_id: uuid::Uuid::new_v4(),
        hook_executable: context.hook_executable,
        data_home_candidates: vec![context.data_home],
        input: CodexHookInput::SessionStart {
            session_id: uuid::Uuid::new_v4().to_string(),
            transcript_path: None,
            cwd: context.cwd,
            model: MODEL.to_owned(),
            permission_mode: PERMISSION.to_owned(),
            source: SOURCE.to_owned(),
        },
    }
}
