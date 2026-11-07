use super::super::deadline;
use std::path::Path;
use yolo_tui_tab::{TAB_FILE, TabDocument};

pub fn read(path: &Path) -> TabDocument {
    yolo_runtime::paths::read_json(&path.join(TAB_FILE)).unwrap()
}
pub fn ratio(document: &TabDocument) -> u16 {
    let yolo_presentation_layout::Tree::Split { ratio, .. } = document
        .tabs
        .active()
        .view        .layout
        .document()
        .root
    else {
        panic!()
    };
    ratio.into()
}
pub async fn wait(path: &Path, ready: impl Fn(&TabDocument) -> bool) -> TabDocument {
    tokio::time::timeout_at(deadline(), async {
        loop {
            let saved = read(path);
            if ready(&saved) {
                return saved;
            }
            tokio::time::sleep(super::POLL).await;
        }
    })
    .await
    .unwrap()
}
pub fn label(screen: &vt100::Screen, label: &str) -> Option<(u16, u16)> {
    let (rows, columns) = screen.size();
    let width = u16::try_from(label.len()).ok()?;
    for row in 0..rows {
        for column in 0..columns {
            if column.checked_add(width)? > columns {
                break;
            }
            let matches = label.chars().enumerate().all(|(index, character)| {
                screen
                    .cell(row, column + u16::try_from(index).unwrap())
                    .is_some_and(|cell| cell.contents() == character.to_string())
            });
            if matches {
                return Some((column, row));
            }
        }
    }
    None
}
