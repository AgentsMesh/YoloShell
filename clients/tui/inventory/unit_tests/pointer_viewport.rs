use super::{constants::*, pointer_support::*};
use ratatui::layout::Rect;
use std::collections::BTreeMap;
use yolo_tui_inventory::{InventoryHit, View};

#[test]
fn each_painted_row_maps_to_its_rendered_identity_after_selection_scrolls_the_list() {
    let (mut inventory, terminals) = catalogue();
    let mut session = None;
    for view in [View::Terminals, View::Tree] {
        inventory.view = view;
        let selected = *inventory.visible_selections("", &terminals).last().unwrap();
        assert!(inventory.select(selected, &mut session, "", &terminals));
        let viewport = render(
            &inventory,
            &terminals,
            session,
            "",
            Rect {
                height: POINTER_SHORT_HEIGHT,
                ..area()
            },
        );
        let rows = viewport.rows_area();
        let hits = (rows.y..rows.bottom())
            .filter_map(|y| viewport.hit(rows.x, y))
            .collect::<Vec<_>>();
        assert!(hits.contains(&InventoryHit::Row(selected)));
        assert_eq!(viewport.hit(rows.right(), rows.y), None);
        assert_eq!(viewport.hit(rows.x, rows.bottom()), None);
        assert!(viewport.current(&inventory, &terminals, session, ""));
    }
}

#[test]
fn the_context_row_and_empty_rows_have_no_hit_targets() {
    let (inventory, terminals) = catalogue();
    let viewport = render(&inventory, &terminals, None, "", area());
    assert!(
        (viewport.area().x..viewport.area().right())
            .all(|x| viewport.hit(x, viewport.area().y).is_none())
    );
}

#[test]
fn empty_unsupported_and_unpainted_list_tail_have_no_row_hits() {
    let (mut inventory, terminals) = catalogue();
    for view in [View::Terminals, View::Tree] {
        inventory.view = view;
        let viewport = render(&inventory, &terminals, None, NONMATCHING_FILTER, area());
        let rows = viewport.rows_area();
        assert!((rows.y..rows.bottom()).all(|y| viewport.hit(rows.x, y).is_none()));
        assert_eq!(viewport.scroll_target(true), None);
    }
    let none = BTreeMap::new();
    inventory.replace(None, None);
    inventory.view = View::Tree;
    let viewport = render(&inventory, &none, None, "", area());
    assert_eq!(
        viewport.hit(viewport.rows_area().x, viewport.rows_area().y),
        None
    );
    inventory.view = View::Terminals;
    let viewport = render(
        &inventory,
        &none,
        None,
        "",
        Rect {
            height: POINTER_SHORT_HEIGHT,
            ..area()
        },
    );
    assert_eq!(
        viewport.hit(
            viewport.rows_area().x,
            viewport.rows_area().bottom() - POINTER_ROW_STEP
        ),
        None
    );
    let empty = render(&inventory, &none, None, "", Rect::default());
    assert_eq!(empty.hit(0, 0), None);
}

#[test]
fn collapsing_either_dimension_removes_hit_targets_until_a_real_repaint() {
    let (mut inventory, terminals) = catalogue();
    let mut session = None;
    for view in [View::Terminals, View::Tree] {
        inventory.view = view;
        let selected = inventory.visible_selections("", &terminals)[0];
        assert!(inventory.select(selected, &mut session, "", &terminals));
        for collapsed in [
            Rect { width: 0, ..area() },
            Rect {
                height: 0,
                ..area()
            },
        ] {
            let viewport = render(&inventory, &terminals, session, "", collapsed);
            assert!(viewport.current(&inventory, &terminals, session, ""));
            for y in area().y..area().bottom() {
                assert!((area().x..area().right()).all(|x| viewport.hit(x, y).is_none()));
            }
            let repainted = render(&inventory, &terminals, session, "", area());
            let rows = repainted.rows_area();
            assert_eq!(
                repainted.hit(rows.x, rows.y),
                Some(InventoryHit::Row(selected))
            );
        }
    }
}
