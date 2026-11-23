fn codex_composer_visible(bottom: &str) -> bool {
    let lines = bottom
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let Some((footer, above)) = lines.split_last() else {
        return false;
    };
    footer.contains(CODEX_MODEL_FOOTER)
        && above
            .iter()
            .any(|line| line.contains(CODEX_COMPOSER_PROMPT))
}

fn codex_scroll_keys(event: MouseEventKind) -> Vec<u8> {
    let key = if event == MouseEventKind::ScrollUp {
        LINE_UP
    } else {
        LINE_DOWN
    };
    let mut keys = Vec::with_capacity(CODEX_SCROLL_LINES * key.len());
    for _ in 0..CODEX_SCROLL_LINES {
        keys.extend_from_slice(key);
    }
    keys
}
