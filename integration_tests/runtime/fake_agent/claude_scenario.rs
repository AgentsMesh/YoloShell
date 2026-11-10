use crate::claude_wire::ClaudeHookFrame;
use serde::{Deserialize, Serialize};
use std::error::Error;

pub(crate) const SCENARIO_SCHEMA_VERSION: u32 = 1;
pub(crate) const SCENARIO_FILE_ENV: &str = "YOLO_TEST_CLAUDE_SCENARIO_FILE";
pub(crate) const HOOK_SOCKET_ENV: &str = "YOLO_TEST_CLAUDE_HOOK_SOCKET";
pub(crate) const SCENARIO_SCHEMA_UNSUPPORTED: &str = "unsupported fake Claude scenario schema";
pub(crate) const SCENARIO_FORCED_ERROR: &str = "fake Claude hook delivery failed";

pub(crate) type AdapterResult = Result<(), Box<dyn Error>>;
pub(crate) type ScenarioResult = Result<ScenarioOutcome, Box<dyn Error>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum HookAvailability {
    #[serde(rename = "enabled")]
    Enabled,
    #[serde(rename = "missing")]
    Missing,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "disableAllHooks")]
    DisableAllHooks,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaudeScenario {
    pub(crate) schema_version: u32,
    pub(crate) hook_availability: HookAvailability,
    pub(crate) frames: Vec<ClaudeHookFrame>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ScenarioOutcome {
    Delivered { frame_count: usize },
    Suppressed(HookAvailability),
}

pub(crate) trait ClaudeHookAdapter {
    fn deliver(&mut self, frame: &ClaudeHookFrame) -> AdapterResult;
}

pub(crate) fn execute(
    scenario: &ClaudeScenario,
    adapter: &mut impl ClaudeHookAdapter,
) -> ScenarioResult {
    if scenario.schema_version != SCENARIO_SCHEMA_VERSION {
        return Err(SCENARIO_SCHEMA_UNSUPPORTED.into());
    }
    for frame in &scenario.frames {
        frame.validate().map_err(Box::<dyn Error>::from)?;
    }
    match scenario.hook_availability {
        HookAvailability::Enabled => {
            for frame in &scenario.frames {
                adapter.deliver(frame)?;
            }
            Ok(ScenarioOutcome::Delivered {
                frame_count: scenario.frames.len(),
            })
        }
        HookAvailability::Missing | HookAvailability::DisableAllHooks => {
            Ok(ScenarioOutcome::Suppressed(scenario.hook_availability))
        }
        HookAvailability::Error => Err(SCENARIO_FORCED_ERROR.into()),
    }
}
