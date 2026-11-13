use super::{creation_support::Fixture, drawing_support, finalization_support::*};
use crate::{Mode, constants::*, finalization_status::BLOCKED_LABEL};
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_terminal_protocol::PublicError;

const WIDTH: u16 = 220;
const HEIGHT: u16 = 12;
const FILTER: &str = "active-filter-input";
const DIRECTORY: &str = "/active-directory-input";
const STORAGE_FAILURE: &str = "Fixture storage is unavailable";
const BLOCKED_COUNT: usize = 1;
const CONFIRM_TERMINATE_LABEL: &str = "End selected terminal and its programs?";
const CONFIRM_ARCHIVE_LABEL: &str = "Archive selected terminal?";

#[test]
fn active_prompts_remain_visible_while_confirmation_and_storage_errors_stay_retained() {
    for storage_failure in [false, true] {
        let mut fixture = Fixture::new();
        let (_, creation, request) = confirmed(&mut fixture);
        let token = token(&request);
        let retained = fixture.state.creations.finalizing.get_mut(&token).unwrap();
        // A restored obligation has no live pane/presentation authority.
        retained.tab = None;
        retained.pane = None;
        fixture.state.status = CONNECTED_LABEL.into();
        finish(
            &mut fixture,
            request,
            Err(ControlError::Client(ClientError::Remote(
                PublicError::TargetMismatch,
            ))),
        );
        let rejection = fixture.state.creations.finalizing[&token].rejection.clone();
        if storage_failure {
            fixture.state.creations.persistence_error = Some(STORAGE_FAILURE);
        }
        fixture.state.filter = FILTER.into();
        fixture.state.entry = DIRECTORY.into();
        for (mode, prompt) in [
            (Mode::Search, format!("{SEARCH_LABEL}: {FILTER}")),
            (Mode::Directory, format!("{DIRECTORY_LABEL}: {DIRECTORY}")),
            (Mode::ConfirmTerminate, CONFIRM_TERMINATE_LABEL.into()),
            (Mode::ConfirmArchive, CONFIRM_ARCHIVE_LABEL.into()),
        ] {
            fixture.state.mode = mode;
            let text = drawing_support::render(&fixture.state, WIDTH, HEIGHT).0;
            assert!(text.contains(&prompt));
            if mode != Mode::Terminal {
                assert!(text.contains(&format!("{BLOCKED_LABEL}: {BLOCKED_COUNT}")));
            }
            assert_eq!(text.contains(UNSAVED_SHELL_CHANGES_LABEL), storage_failure);
            assert!(!text.contains(STORAGE_FAILURE));
            assert!(!text.contains(&PublicError::TargetMismatch.to_string()));
            assert_eq!(
                fixture.state.creations.finalizing[&token].record.creation,
                creation
            );
            assert_eq!(
                fixture.state.creations.finalizing[&token].rejection,
                rejection
            );
            assert!(fixture.state.writers.is_empty());
        }
        for mode in [Mode::Inventory, Mode::Terminal] {
            fixture.state.mode = mode;
            let text = drawing_support::render(&fixture.state, WIDTH, HEIGHT).0;
            let expected = if storage_failure {
                STORAGE_FAILURE.to_owned()
            } else {
                PublicError::TargetMismatch.to_string()
            };
            assert!(text.contains(&expected));
            if mode != Mode::Terminal {
                assert!(text.contains(&format!("{BLOCKED_LABEL}: {BLOCKED_COUNT}")));
            }
            assert_eq!(
                crate::finalization_status::blocked(&fixture.state),
                BLOCKED_COUNT
            );
        }
        no_commands(&mut fixture);
    }
}
