use ratatui::style::{Color, Modifier};
use yolo_tui_render::styled_lines;

fn style(sgr: &str) -> ratatui::style::Style {
    let lines = styled_lines(&format!("\x1b[{sgr}mX"));
    lines[0].spans[0].style
}

#[test]
fn extended_and_bright_colors_match_the_terminal_palette() {
    assert_eq!(style("38;5;1").fg, Some(Color::Indexed(1)));
    assert_eq!(style("38;2;1;2;3").fg, Some(Color::Rgb(1, 2, 3)));
    assert_eq!(style("94").fg, Some(Color::Indexed(12)));
    assert_eq!(style("48;5;4").bg, Some(Color::Indexed(4)));
    assert_eq!(style("107").bg, Some(Color::Indexed(15)));
}

#[test]
fn dim_and_reverse_are_applied_and_reset() {
    use ratatui::style::Style;
    assert_eq!(style("2"), Style::default().add_modifier(Modifier::DIM));
    assert_eq!(
        style("7"),
        Style::default().add_modifier(Modifier::REVERSED)
    );
    let reset = style("2;7;22;27");
    assert!(
        !reset
            .add_modifier
            .contains(Modifier::DIM | Modifier::REVERSED)
    );
}
