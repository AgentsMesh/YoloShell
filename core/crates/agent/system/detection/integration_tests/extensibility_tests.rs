use super::extensibility_constants::*;
use super::launch::{ResumeIntent, apply_resume_with_registry};
use super::*;
use crate::model::{DEFAULT_COLS, DEFAULT_ROWS, WorkerLaunchSpec};
use anyhow::{Result, ensure};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;
use yolo_application_support::constants::WORKER_LAUNCH_SCHEMA_VERSION;

const PROCESS_PID: u32 = 73;
const PROCESS_GROUP: i32 = 19;
const PARENT_PID: u32 = 1;
const CONTROLLER_GENERATION: u64 = 1;

struct FutureAdapter {
    kind: AgentKind,
}

impl AgentAdapter for FutureAdapter {
    fn kind(&self) -> AgentKind {
        self.kind.clone()
    }

    fn data_home_environment_variable(&self) -> Option<&'static str> {
        Some(FUTURE_DATA_HOME_ENV)
    }

    fn resolve_data_home(&self, environment: &DetectionEnvironment) -> Option<PathBuf> {
        environment.data_home(FUTURE_DATA_HOME_ENV, FUTURE_DEFAULT_DATA_HOME)
    }

    fn matches_process(&self, process: &ObservedProcess) -> bool {
        process.executable == FUTURE_EXECUTABLE
    }

    fn owns_evidence_path(&self, path: &Path, context: &DetectionContext) -> bool {
        context
            .data_home(&self.kind())
            .is_some_and(|home| path == home.join(FUTURE_EVIDENCE_FILE))
    }

    fn evidence_files(&self, context: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(context
            .data_home(&self.kind())
            .map(|home| vec![home.join(FUTURE_EVIDENCE_FILE)])
            .unwrap_or_default())
    }

    fn inspect_evidence(&self, path: &Path, cwd: &Path) -> Result<Option<AgentEvidence>> {
        let Some(data_home) = path.parent() else {
            return Ok(None);
        };
        Ok(Some(AgentEvidence {
            provider_session_id: FUTURE_SESSION_ID.to_owned(),
            cwd: cwd.to_path_buf(),
            path: path.to_path_buf(),
            data_home: data_home.to_path_buf(),
            state: AgentState::WaitingForInput,
        }))
    }

    fn resume_plan(&self, binding: &AgentBinding) -> ResumePlan {
        ResumePlan {
            executable: FUTURE_EXECUTABLE.to_owned(),
            args: vec![
                FUTURE_RESUME_ARGUMENT.to_owned(),
                binding.provider_session_id.clone(),
            ],
            environment: [(FUTURE_DATA_HOME_ENV.to_owned(), binding.data_home.clone())].into(),
        }
    }

    fn validate_binding(&self, binding: &AgentBinding, _: &DetectionContext) -> Result<()> {
        ensure!(binding.kind == self.kind(), FUTURE_KIND_MISMATCH);
        Ok(())
    }
}

fn registry() -> AgentRegistry {
    AgentRegistry::from_adapters(vec![Box::new(future_adapter())]).unwrap()
}

fn future_adapter() -> FutureAdapter {
    FutureAdapter {
        kind: AgentKind::from_name(FUTURE_KIND),
    }
}

fn launch(cwd: &Path) -> WorkerLaunchSpec {
    WorkerLaunchSpec {
        schema_version: WORKER_LAUNCH_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        socket_path: cwd.join(FUTURE_WORKER_SOCKET_FILE),
        auth_token: FUTURE_AUTH_TOKEN.to_owned(),
        controller_generation: CONTROLLER_GENERATION,
        command: FUTURE_SHELL.to_owned(),
        args: Vec::new(),
        cwd: cwd.to_path_buf(),
        environment: Default::default(),
        initial_input: None,
        cols: DEFAULT_COLS,
        rows: DEFAULT_ROWS,
        context: None,
    }
}

#[test]
fn third_adapter_drives_detection_and_resume_without_core_provider_fields() {
    assert!(
        registry()
            .process_environment_keys()
            .contains(&FUTURE_DATA_HOME_ENV)
    );
    let directory = tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let cwd = root.join(FUTURE_PROJECT_DIRECTORY);
    let data_home = root.join(FUTURE_DATA_HOME_DIRECTORY);
    std::fs::create_dir_all(&cwd).unwrap();
    std::fs::create_dir_all(&data_home).unwrap();
    let evidence = data_home.join(FUTURE_EVIDENCE_FILE);
    std::fs::write(&evidence, FUTURE_SESSION_ID).unwrap();
    let variables = [(
        OsString::from(FUTURE_DATA_HOME_ENV),
        data_home.clone().into(),
    )];
    let environment = DetectionEnvironment::from_values(variables, &Default::default());
    let context = registry()
        .detection_context_with_environment(Uuid::new_v4(), cwd.clone(), &environment)
        .unwrap();
    let process = ObservedProcess {
        pid: PROCESS_PID,
        start_time: u64::from(PROCESS_PID),
        parent_pid: PARENT_PID,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: FUTURE_EXECUTABLE.to_owned(),
        argv: vec![FUTURE_EXECUTABLE.to_owned()],
        environment: None,
        cwd: Some(cwd.clone()),
        evidence_paths: Some(vec![evidence]),
    };
    let mut detector = AgentDetector::new(context.clone(), registry(), None).unwrap();
    let DetectionResult::BindingChanged(binding) = detector.detect(&[process]).unwrap() else {
        panic!("{FUTURE_DETECTION_FAILURE}");
    };
    assert_eq!(binding.kind.as_str(), FUTURE_KIND);
    registry().validate_binding(&binding, &context).unwrap();
    let mut launch = launch(&cwd);
    apply_resume_with_registry(&mut launch, &binding, ResumeIntent::Automatic, &registry())
        .unwrap();
    assert_eq!(
        launch.environment.get(FUTURE_DATA_HOME_ENV),
        Some(&data_home.display().to_string())
    );
    let expected_suffix = format!(
        "{}{}",
        FUTURE_SESSION_ID,
        crate::constants::COMMAND_LINE_ENDING
    );
    assert!(
        launch
            .initial_input
            .unwrap()
            .ends_with(expected_suffix.as_bytes())
    );
}

#[path = "extensibility_registry_tests.rs"]
mod registry_tests;
