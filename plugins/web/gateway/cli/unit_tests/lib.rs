use std::ffi::OsString;
use yolo_web_cli::{run_from, selects};
mod constants;
use constants::LISTEN_OPTION;

#[test]
fn dispatch_selects_only_the_explicit_web_subcommand() {
    for (command, expected) in [("web", true), ("tui", false), ("plugins", false)] {
        assert_eq!(selects(&[OsString::from("yolo"), command.into()]), expected);
    }
    assert!(!selects(&[]));
}

#[test]
fn missing_private_credentials_fail_before_listener_startup() {
    assert!(run_from(["yolo", "web"].into_iter().map(OsString::from)).is_err());
    assert!(
        run_from(
            ["yolo", "web", LISTEN_OPTION, "0.0.0.0:80"]
                .into_iter()
                .map(OsString::from)
        )
        .is_err()
    );
}
