use super::super::*;

const DIAGNOSTIC_SCROLLBACK_ROWS: usize = 0;
const TERMINAL_DIAGNOSTIC_LIMIT: usize = 4 * 1_024;

pub(super) async fn capture(
    client: &Client,
    terminal_id: Uuid,
    provider: ProviderFixture,
) -> String {
    if provider.kind != yolo_runtime::agent::codex::constants::KIND {
        return String::new();
    }
    match client.snapshot(terminal_id).await {
        Ok(snapshot) => {
            let mut terminal =
                vt100::Parser::new(snapshot.rows, snapshot.cols, DIAGNOSTIC_SCROLLBACK_ROWS);
            terminal.process(&snapshot.data);
            let contents: String = terminal
                .screen()
                .contents()
                .chars()
                .take(TERMINAL_DIAGNOSTIC_LIMIT)
                .collect();
            format!(
                "codex_runtime_epoch={} terminal={contents:?}",
                snapshot.runtime_epoch
            )
        }
        Err(error) => format!("codex_terminal_unavailable={error}"),
    }
}
