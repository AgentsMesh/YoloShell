use super::ingress_live_source::LiveProcessSource;
use super::support::Fixture;
use crate::agent::{ObservedProcess, ProcessSource};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const ASSET_DIRECTORY: &str = "codex-ingress-assets";
const CWD_DIRECTORY: &str = "project";
const DATA_HOME_DIRECTORY: &str = "codex-home";
const HOOK_FILE: &str = "codex-hook";
const LIVE_PEER_FIXTURE_ENV: &str = "YOLO_CODEX_LIVE_PEER_FIXTURE";
const LIVE_PEER_FIXTURE_MISSING: &str = "YOLO_CODEX_LIVE_PEER_FIXTURE must name the native fixture";
const PROCESS_GROUP: i32 = 7_101;

pub(super) struct IngressTestAssets {
    cwd: PathBuf,
    data_home: PathBuf,
    hook: PathBuf,
    source: Arc<LiveProcessSource>,
}

impl IngressTestAssets {
    pub(super) fn new(root: &Path) -> Self {
        let root = directory(&root.join(ASSET_DIRECTORY));
        Self {
            cwd: directory(&root.join(CWD_DIRECTORY)),
            data_home: directory(&root.join(DATA_HOME_DIRECTORY)),
            hook: fixture_executable(&root.join(HOOK_FILE)),
            source: Arc::new(LiveProcessSource::default()),
        }
    }

    pub(super) fn request(&self, fixture: &Fixture) -> crate::codex_hooks::CodexAdmissionContext {
        crate::codex_hooks::CodexAdmissionContext::new(
            fixture.identity.clone(),
            self.cwd.clone(),
            self.data_home.clone(),
            self.hook.clone(),
            std::env::current_exe().unwrap().canonicalize().unwrap(),
        )
    }

    pub(super) fn process_source(
        &self,
        fixture: &Fixture,
        shell: crate::process::ProcessIdentity,
        _socket: &Path,
    ) -> Arc<dyn ProcessSource> {
        let request = self.request(fixture);
        let environment = BTreeMap::from([
            (
                crate::codex_hooks::TERMINAL_ENV.to_owned(),
                Fixture::terminal_id().to_string(),
            ),
            (
                crate::codex_hooks::RUNTIME_ENV.to_owned(),
                Fixture::runtime_epoch().to_string(),
            ),
            (
                crate::agent::codex::constants::DATA_HOME_ENV.to_owned(),
                self.data_home.to_string_lossy().into_owned(),
            ),
        ]);
        self.source.install(
            ObservedProcess {
                pid: fixture.process.pid,
                start_time: fixture.process.start_time,
                parent_pid: shell.pid,
                process_group: PROCESS_GROUP,
                foreground_group: PROCESS_GROUP,
                executable: request.real_executable.to_string_lossy().into_owned(),
                argv: vec![request.real_executable.to_string_lossy().into_owned()],
                environment: Some(environment),
                cwd: Some(self.cwd.clone()),
                evidence_paths: None,
            },
            self.hook.clone(),
        );
        self.source.clone()
    }

    pub(super) fn live_source(&self) -> Arc<LiveProcessSource> {
        self.source.clone()
    }
    pub(super) fn hook_executable(&self) -> &Path {
        &self.hook
    }
}

fn directory(path: &Path) -> PathBuf {
    std::fs::create_dir(path).unwrap();
    path.canonicalize().unwrap()
}

fn fixture_executable(path: &Path) -> PathBuf {
    let fixture = std::env::var_os(LIVE_PEER_FIXTURE_ENV).expect(LIVE_PEER_FIXTURE_MISSING);
    std::fs::copy(fixture, path).unwrap();
    std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(crate::constants::RUNTIME_EXECUTABLE_MODE),
    )
    .unwrap();
    path.canonicalize().unwrap()
}
