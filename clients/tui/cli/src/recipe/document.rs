//! Recipe file schema and validation. The document is user-facing JSON, not
//! private IPC; unknown fields are rejected so a typo cannot be ignored.

use crate::constants::*;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use uuid::Uuid;
use yolo_hub_agent_protocol::Prompt;
use yolo_hub_agent_run_protocol::validate_labels;

use super::prompt::RecipePromptWait;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeDocument {
    pub schema_version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<Uuid>,
    /// Absolute working directory. Optional when a workspace is set; the Hub
    /// then uses the workspace root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// Optional parent run name or UUID recorded on every started run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// Stop agents started earlier in this recipe when a later start fails.
    #[serde(default)]
    pub stop_on_failure: bool,
    /// Labels applied to every started run; an agent's labels override these.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    pub agents: Vec<RecipeAgent>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeAgent {
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Stable idempotency token; supply it to make a retry replay-safe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_token: Option<Uuid>,
    /// Labels for this run; overrides the document-level labels.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,
    /// Agent names that must start before this agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<String>,
    /// Hold this start until every `after` run reaches the observed state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait: Option<RecipeWait>,
    /// Deliver this prompt once the started run is promptable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<RecipePrompt>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipePrompt {
    pub text: String,
    #[serde(default = "default_prompt_timeout_seconds")]
    pub timeout_seconds: u32,
    #[serde(default)]
    pub wait: RecipePromptWait,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecipeWait {
    pub until: RecipeWaitUntil,
    #[serde(default = "default_wait_timeout_seconds")]
    pub timeout_seconds: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RecipeWaitUntil {
    Promptable,
    Idle,
    Exited,
}

impl RecipeWaitUntil {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Promptable => RECIPE_WAIT_PROMPTABLE,
            Self::Idle => RECIPE_WAIT_IDLE,
            Self::Exited => RECIPE_WAIT_EXITED,
        }
    }
}

const fn default_wait_timeout_seconds() -> u32 {
    DEFAULT_RECIPE_WAIT_SECONDS
}

const fn default_prompt_timeout_seconds() -> u32 {
    DEFAULT_RECIPE_PROMPT_SECONDS
}

pub(crate) fn read(path: &Path) -> Result<RecipeDocument> {
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > MAX_RECIPE_BYTES {
        bail!(INVALID_RECIPE);
    }
    let bytes = std::fs::read(path)?;
    let document: RecipeDocument =
        serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!(INVALID_RECIPE))?;
    validate(&document)?;
    Ok(document)
}

fn validate(document: &RecipeDocument) -> Result<()> {
    if document.schema_version != RECIPE_SCHEMA_VERSION {
        bail!(INVALID_RECIPE_SCHEMA);
    }
    if document.agents.is_empty() || document.agents.len() > MAX_RECIPE_AGENTS {
        bail!(INVALID_RECIPE_AGENTS);
    }
    if document.workspace.is_none() && document.cwd.is_none() {
        bail!(INVALID_RECIPE_CWD);
    }
    if validate_labels(&document.labels).is_err() {
        bail!(INVALID_RECIPE_LABELS);
    }
    if let Some(cwd) = document.cwd.as_deref() {
        let path = Path::new(cwd);
        if !path.is_absolute()
            || cwd.len() > MAX_RECIPE_PATH_BYTES
            || cwd.chars().any(char::is_control)
        {
            bail!(INVALID_RECIPE_CWD);
        }
    }
    let mut names = std::collections::BTreeSet::new();
    for agent in &document.agents {
        if !valid_text(&agent.provider, MAX_RECIPE_PROVIDER_BYTES) {
            bail!(INVALID_RECIPE_PROVIDER);
        }
        if let Some(name) = agent.name.as_deref()
            && (!valid_text(name, MAX_RECIPE_NAME_BYTES)
                || !names.insert(name.to_ascii_lowercase()))
        {
            bail!(INVALID_RECIPE_NAME);
        }
        if agent.start_token.is_some_and(|token| token.is_nil()) {
            bail!(INVALID_RECIPE_TOKEN);
        }
        if validate_labels(&agent.labels).is_err() {
            bail!(INVALID_RECIPE_LABELS);
        }
        if let Some(wait) = &agent.wait
            && (agent.after.is_empty()
                || wait.timeout_seconds == 0
                || wait.timeout_seconds > MAX_RECIPE_WAIT_SECONDS)
        {
            bail!(INVALID_RECIPE_WAIT);
        }
        if let Some(prompt) = &agent.prompt
            && (prompt.timeout_seconds == 0
                || prompt.timeout_seconds > MAX_RECIPE_PROMPT_SECONDS
                || Prompt::new(prompt.text.clone()).is_err())
        {
            bail!(INVALID_RECIPE_PROMPT);
        }
        let merged = crate::recipe::merged(&document.labels, &agent.labels);
        if validate_labels(&merged).is_err() {
            bail!(INVALID_RECIPE_LABELS);
        }
    }
    crate::recipe::compute(&document)?;
    if let Some(parent) = document.parent.as_deref()
        && (parent.is_empty()
            || parent.len() > MAX_RECIPE_NAME_BYTES
            || parent.chars().any(char::is_control))
    {
        bail!(INVALID_RECIPE_PARENT);
    }
    Ok(())
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
