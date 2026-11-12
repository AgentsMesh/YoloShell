use std::path::Path;
use yolo_agent_contract::AgentBinding;
use yolo_agent_provider_runtime::{TurnEndKind, TurnEndProbe, TurnEndRecord};

#[derive(Debug, Default)]
pub struct CodexTurnEndProbe;

impl TurnEndProbe for CodexTurnEndProbe {
    fn read_turn_end(&self, binding: &AgentBinding) -> anyhow::Result<Option<TurnEndRecord>> {
        let end = yolo_codex::CodexAdapter.read_turn_end(
            Path::new(&binding.evidence_path),
            Path::new(&binding.data_home),
        )?;
        Ok(end.map(|end| TurnEndRecord {
            turn_id: end.turn_id,
            kind: match end.kind {
                yolo_codex::TurnEndKind::Completed => TurnEndKind::Completed,
                yolo_codex::TurnEndKind::Aborted => TurnEndKind::Aborted,
            },
        }))
    }
}
