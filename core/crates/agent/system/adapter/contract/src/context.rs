use crate::AgentAdapter;
use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use yolo_agent_contract::{AgentKind, ObservedProcess};

pub type LaunchEnvironment = BTreeMap<String, String>;

#[derive(Debug, Clone)]
pub struct DetectionContext {
    pub terminal_id: Uuid,
    pub expected_cwd: PathBuf,
    pub data_homes: BTreeMap<AgentKind, PathBuf>,
}

impl DetectionContext {
    pub fn new(
        terminal_id: Uuid,
        expected_cwd: PathBuf,
        data_homes: impl IntoIterator<Item = (AgentKind, PathBuf)>,
    ) -> Self {
        Self {
            terminal_id,
            expected_cwd,
            data_homes: data_homes.into_iter().collect(),
        }
    }

    pub fn data_home(&self, kind: &AgentKind) -> Option<&Path> {
        self.data_homes.get(kind).map(PathBuf::as_path)
    }

    pub fn for_process(&self, adapter: &dyn AgentAdapter, process: &ObservedProcess) -> Self {
        let Some(variables) = process.environment.as_ref() else {
            return self.clone();
        };
        let environment = DetectionEnvironment::from_strings(variables);
        let mut context = self.clone();
        let kind = adapter.kind();
        context.data_homes.remove(&kind);
        if let Some(home) = adapter
            .resolve_data_home(&environment)
            .and_then(|home| canonical_process_data_home(&home))
        {
            context.data_homes.insert(kind, home);
        }
        context
    }

    pub fn with_binding_data_home(&self, kind: AgentKind, home: PathBuf) -> Self {
        let mut context = self.clone();
        context.data_homes.insert(kind, home);
        context
    }
}

pub struct DetectionEnvironment {
    variables: BTreeMap<OsString, OsString>,
}

impl DetectionEnvironment {
    pub fn current(overrides: &LaunchEnvironment) -> Self {
        Self::from_values(std::env::vars_os(), overrides)
    }

    pub fn variable(&self, name: &str) -> Option<PathBuf> {
        self.variables.get(OsStr::new(name)).map(PathBuf::from)
    }

    pub fn data_home(&self, variable: &str, default_directory: &str) -> Option<PathBuf> {
        let path = self
            .variables
            .get(OsStr::new(variable))
            .map(PathBuf::from)
            .or_else(|| {
                self.variables
                    .get(OsStr::new(crate::constants::HOME_ENV))
                    .map(PathBuf::from)
                    .map(|home| home.join(default_directory))
            })?;
        if !unambiguous_data_home(&path) {
            return None;
        }
        Some(canonical_existing_directory(&path).unwrap_or(path))
    }

    pub fn from_strings(variables: &BTreeMap<String, String>) -> Self {
        Self {
            variables: variables
                .iter()
                .map(|(key, value)| (OsString::from(key), OsString::from(value)))
                .collect(),
        }
    }

    pub fn from_values(
        variables: impl IntoIterator<Item = (OsString, OsString)>,
        overrides: &LaunchEnvironment,
    ) -> Self {
        let mut variables: BTreeMap<_, _> = variables.into_iter().collect();
        variables.extend(
            overrides
                .iter()
                .map(|(key, value)| (OsString::from(key), OsString::from(value))),
        );
        Self { variables }
    }
}

fn canonical_process_data_home(path: &Path) -> Option<PathBuf> {
    if !unambiguous_data_home(path) {
        return None;
    }
    canonical_existing_directory(path)
}

fn unambiguous_data_home(path: &Path) -> bool {
    use std::path::Component;
    path.is_absolute()
        && !path.as_os_str().is_empty()
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir))
}

fn canonical_existing_directory(path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.is_dir().then_some(canonical)
}
