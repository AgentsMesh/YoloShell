use super::{Driver, constants::*};

pub async fn begin(driver: &mut Driver) -> (u16, u16) {
    driver.wait(|text| text.contains(SOURCE_MARKER)).await;
    let screen = driver.screen();
    let (rows, columns) = screen.size();
    let width = SOURCE_MARKER.chars().count() as u16;
    let (column, row) = (0..rows)
        .flat_map(|row| (0..=columns.saturating_sub(width)).map(move |column| (column, row)))
        .find(|(column, row)| {
            SOURCE_MARKER
                .chars()
                .enumerate()
                .all(|(offset, character)| {
                    screen
                        .cell(*row, *column + offset as u16)
                        .is_some_and(|cell| cell.contents() == character.to_string())
                })
        })
        .unwrap();
    let start = column + SELECTED_PREFIX_COLUMNS;
    mouse(driver, MOUSE_SHIFT_LEFT, start, row, MOUSE_DOWN).await;
    driver.wait(|text| text.contains(SELECTING_LABEL)).await;
    (start, row)
}

pub async fn finish(driver: &mut Driver, (column, row): (u16, u16)) {
    let end = column + SELECTED_COLUMNS - COORDINATE_BASE;
    mouse(driver, MOUSE_SHIFT_DRAG, end, row, MOUSE_DOWN).await;
    mouse(driver, MOUSE_RELEASE, end, row, MOUSE_UP).await;
    driver.wait(|text| text.contains(SELECTED_LABEL)).await;
}

pub async fn mouse(driver: &Driver, button: u16, column: u16, row: u16, suffix: char) {
    driver
        .send(
            format!(
                "{MOUSE_PREFIX}{button};{};{}{suffix}",
                column + COORDINATE_BASE,
                row + COORDINATE_BASE
            )
            .as_bytes(),
        )
        .await;
}
