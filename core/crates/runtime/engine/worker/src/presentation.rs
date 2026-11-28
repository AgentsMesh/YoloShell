use crate::{model::TerminalPresentation, paths::Paths};
use uuid::Uuid;

pub fn load_presentation(paths: &Paths, terminal_id: Uuid, epoch: Uuid) -> TerminalPresentation {
    let title = super::load_snapshot(paths, terminal_id)
        .filter(|snapshot| snapshot.runtime_epoch == epoch)
        .and_then(|snapshot| yolo_terminal_metadata::title_from_snapshot(&snapshot.data));
    TerminalPresentation {
        title,
        ..Default::default()
    }
}
