use yolo_tui_render::elide;

#[test]
fn elision_reserves_cells_for_the_ellipsis_and_wide_glyphs() {
    assert_eq!(elide("short", 8), "short");
    assert_eq!(elide("a-long-terminal-title", 8), "a-long-…");
    assert_eq!(elide("界界界", 4), "界…");
    assert_eq!(elide("abc", 0), "");
}
