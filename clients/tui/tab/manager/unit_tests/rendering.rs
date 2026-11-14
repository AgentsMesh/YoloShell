use super::fixtures::*;
use crate::{Manager, constants::*};
use crossterm::event::KeyCode;

#[test]
fn list_active_marker_and_each_edit_prompt_are_rendered_with_errors() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    let text = rendered(&manager, &catalog, FRAME_WIDTH, FRAME_HEIGHT);
    for label in [TITLE, HELP, SECOND_NAME, THIRD_NAME] {
        assert!(text.contains(label));
    }
    assert!(text.contains(&format!("{ACTIVE_MARK}{}", catalog.active().name)));
    manager.create();
    paste(&mut manager, &catalog, NEW_NAME);
    assert!(
        rendered(&manager, &catalog, FRAME_WIDTH, FRAME_HEIGHT)
            .contains(&format!("{CREATE_LABEL}{NEW_NAME}"))
    );
    press(&mut manager, &catalog, KeyCode::Esc);
    manager.rename(&catalog);
    assert!(rendered(&manager, &catalog, FRAME_WIDTH, FRAME_HEIGHT).contains(RENAME_LABEL));
    press(&mut manager, &catalog, KeyCode::Esc);
    press(&mut manager, &catalog, KeyCode::Char(REMOVE_KEY));
    manager.error(ERROR_MESSAGE.into());
    let text = rendered(&manager, &catalog, FRAME_WIDTH, FRAME_HEIGHT);
    assert!(text.contains(REMOVE_LABEL));
    assert!(text.contains(ERROR_MESSAGE));
}

#[test]
fn tiny_or_empty_viewports_render_every_form_without_mutating_selected_view() {
    let catalog = catalog();
    let mut manager = Manager::new(&catalog);
    for command in [None, Some(CREATE_KEY), Some(RENAME_KEY), Some(REMOVE_KEY)] {
        press(&mut manager, &catalog, KeyCode::Esc);
        if let Some(command) = command {
            press(&mut manager, &catalog, KeyCode::Char(command));
        }
        manager.error(ERROR_MESSAGE.into());
        for (width, height) in SMALL_FRAMES {
            let _ = rendered(&manager, &catalog, *width, *height);
        }
        empty_area(&manager, &catalog);
    }
}
