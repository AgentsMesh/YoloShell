const SCROLLBACK_LINES: usize = 100;
const LINE: &[u8] = b"scrollback\r\n";
const ALT_SCREEN: &[u8] = b"\x1b[?1049hCodex-like alternate screen\r\n";
const PAGE_UP: &[u8] = b"\x1b[5~";
const PAGE_DOWN: &[u8] = b"\x1b[6~";
const CODEX_UP: &[u8] = b"\x1b[A\x1b[A\x1b[A";
const CODEX_DOWN: &[u8] = b"\x1b[B\x1b[B\x1b[B";
const CODEX_OPEN_UP: &[u8] = b"\x14\x1b[A\x1b[A\x1b[A";
fn codex_main() -> Vec<u8> {
    codex_screen("› Ask Codex to do anything\r\n\r\n  gpt-6-astra max · ~/Works/Hello\r\n")
}

fn codex_draft() -> Vec<u8> {
    codex_screen("› hello\r\n\r\n  gpt-6-astra max · ~/Works/Hello\r\n")
}

fn codex_screen(text: &str) -> Vec<u8> {
    // Fill past the 30-row fixture so Codex's frame sits at the live tail and
    // the test can also exercise local scrollback.
    let mut data = "history\r\n".repeat(40).into_bytes();
    data.extend_from_slice(text.as_bytes());
    data
}
const CODEX_TRANSCRIPT: &[u8] = b"/ T R A N S C R I P T / / / / / / / / / /\r\nhistory\r\n";
