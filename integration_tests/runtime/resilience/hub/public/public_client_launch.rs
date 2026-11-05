use super::{constants::*, fixture::Fixture};
use crate::hub::tui_command::TERMINALS_COMMAND;
use std::path::Path;

fn quote(path: &Path) -> String {
    path.to_str().unwrap().replace('\'', QUOTE_ESCAPE)
}

pub fn launch(fixture: &Fixture, workspace: &Path) -> (String, String) {
    let binary = crate::bazel_runfile(&std::env::var(TUI_BINARY_ENV).unwrap()).unwrap();
    let nonce = uuid::Uuid::new_v4();
    let command = format!(
        "{CLIENT_STATE_ENV}='{}' '{}' {TERMINALS_COMMAND} {PROFILE_ARGUMENT} '{}' {ACCOUNT_KEY_ARGUMENT} '{}' {CLIENT_KEY_ARGUMENT} '{}'; printf '%s%s\\n' '{COMPLETE_PREFIX}' '{nonce}'\n",
        quote(workspace),
        quote(&binary),
        quote(&fixture.profile_path),
        quote(fixture.sshd.identity_path()),
        quote(&fixture.key_path),
    );
    (command, format!("{COMPLETE_PREFIX}{nonce}"))
}

const CLIENT_STATE_ENV: &str = "YOLO_TUI_STATE_DIRECTORY";
