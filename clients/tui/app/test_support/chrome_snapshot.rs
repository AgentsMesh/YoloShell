use ratatui::{Terminal, backend::TestBackend, style::Color};
use std::fmt::Write;

const OUTPUT_DIRECTORY_ENV: &str = "TEST_UNDECLARED_OUTPUTS_DIR";
const CELL_WIDTH: u16 = 9;
const CELL_HEIGHT: u16 = 18;
const BASELINE: u16 = 14;
const BACKGROUND: &str = "#111418";
const FOREGROUND: &str = "#d6dbe1";

/// Review artifact from the actual terminal buffer; never opens or focuses a window.
pub fn save(terminal: &Terminal<TestBackend>, name: &str) {
    let Some(root) = std::env::var_os(OUTPUT_DIRECTORY_ENV) else {
        return;
    };
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width * CELL_WIDTH;
    let height = buffer.area.height * CELL_HEIGHT;
    let mut svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"><rect width=\"100%\" height=\"100%\" fill=\"{BACKGROUND}\"/><g font-family=\"Menlo,DejaVu Sans Mono,monospace\" font-size=\"14\">"
    );
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            if cell.symbol().trim().is_empty() {
                continue;
            }
            let color = match cell.fg {
                Color::Green => "#80c98d",
                Color::Cyan => "#73b7c7",
                Color::Yellow => "#e1c27b",
                Color::DarkGray => "#70808f",
                _ => FOREGROUND,
            };
            write!(
                svg,
                "<text x=\"{}\" y=\"{}\" fill=\"{color}\">{}</text>",
                x * CELL_WIDTH,
                y * CELL_HEIGHT + BASELINE,
                escape(cell.symbol())
            )
            .unwrap();
        }
    }
    svg.push_str("</g></svg>");
    std::fs::write(std::path::PathBuf::from(root).join(name), svg).unwrap();
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
