use super::pointer_support::*;
use yolo_tui_inventory::{InventorySelection, View};

#[test]
fn explicit_row_selection_preserves_other_views_and_rejects_wrong_view_or_filter() {
    let (mut inventory, terminals) = catalogue();
    let mut session = None;
    for view in [View::Terminals, View::Agents, View::Machines] {
        inventory.view = view;
        let visible = inventory.visible_selections("", &terminals);
        let selected = *visible.last().unwrap();
        assert!(inventory.select(selected, &mut session, "", &terminals));
        assert_eq!(inventory.selected(session), Some(selected));
        assert!(!inventory.select(
            selected,
            &mut session,
            super::constants::NONMATCHING_FILTER,
            &terminals
        ));
        let other = match view {
            View::Terminals => InventorySelection::Agent(*inventory.agents.keys().next().unwrap()),
            _ => InventorySelection::Terminal(*terminals.keys().next().unwrap()),
        };
        assert!(!inventory.select(other, &mut session, "", &terminals));
        assert_eq!(inventory.scope, None);
    }
}

#[test]
fn scrolling_moves_one_visible_selection_and_clamps_at_the_ends() {
    let (mut inventory, terminals) = catalogue();
    let mut session = None;
    for view in [View::Terminals, View::Agents, View::Machines] {
        inventory.view = view;
        let visible = inventory.visible_selections("", &terminals);
        assert!(inventory.select(visible[0], &mut session, "", &terminals));
        let viewport = render(&inventory, &terminals, session, "", area());
        assert_eq!(viewport.scroll_target(false), Some(visible[0]));
        assert_eq!(
            viewport.scroll_target(true),
            Some(visible[super::constants::POINTER_ROW_STEP as usize])
        );
        let last = *visible.last().unwrap();
        assert!(inventory.select(last, &mut session, "", &terminals));
        assert_eq!(
            render(&inventory, &terminals, session, "", area()).scroll_target(true),
            Some(last)
        );
    }
}
