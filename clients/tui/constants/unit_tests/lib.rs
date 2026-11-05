use yolo_tui_app_constants::parsed_self_terminal;

const SELF: &str = "4f0a7b47-4b7d-4d02-bb1a-d7ee285f3642";

#[test]
fn matches_only_the_terminal_named_by_the_worker_environment() {
    let self_id = SELF.parse().unwrap();
    assert!(parsed_self_terminal(Some(SELF), self_id));
    assert!(!parsed_self_terminal(
        Some("11111111-2222-4333-8444-555555555555"),
        self_id
    ));
    assert!(!parsed_self_terminal(None, self_id));
    assert!(!parsed_self_terminal(Some("not-a-uuid"), self_id));
}
