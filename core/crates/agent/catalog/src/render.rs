use yolo_hub_agent_run_protocol::AgentRunSummary;

pub fn text(run: &AgentRunSummary) -> String {
    let terminal = run
        .terminal
        .map(|terminal| terminal.terminal_id.to_string())
        .unwrap_or_else(|| "-".into());
    format!(
        "{} {:?} provider={} terminal={} revision={} run={}",
        run.name, run.state, run.provider_id, terminal, run.revision, run.address.run_id
    )
}

pub fn details(run: &AgentRunSummary) -> String {
    let mut lines = vec![text(run)];
    if let Some(parent) = run.parent {
        lines.push(format!("parent={}", parent.run_id));
    }
    if let Some(workspace) = run.workspace {
        lines.push(format!("workspace={}", workspace.workspace_id));
    }
    if let Some(current) = run.current {
        lines.push(format!("current={current}"));
    }
    lines.push(format!(
        "created_at_ms={} updated_at_ms={}",
        run.created_at_ms, run.updated_at_ms
    ));
    lines.join("\n")
}
