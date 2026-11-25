use crate::{
    commands::*,
    fixtures::{constants::*, *},
};
use yolo_hub_file_protocol::{FileRootSummary, MAX_FILE_ROOT_PAGE, ReadFileResult};
use yolo_hub_identity::{FileRootEpoch, FileRootId};
use yolo_hub_rpc_protocol::{Command, Reply, Response};
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn file_operations_are_read_only_and_do_not_become_local_configuration_commands() {
    for command in [
        Command::ListFileRoots { after: None },
        Command::ReadFile {
            request: file_read(),
        },
    ] {
        assert!(command.requires_read_reply_admission());
        assert!(!command.mutates());
        assert!(!command.local_only());
        assert_eq!(command.request_id(), None);
    }
    assert!(!Command::Hello {}.requires_read_reply_admission());
    assert!(!Command::GetConfiguration {}.requires_read_reply_admission());
}

#[test]
fn root_pages_reject_mixed_origins_epochs_duplicate_order_and_bad_cursors() {
    let root = file_root();
    let epoch = root.fields().target.epoch;
    let listing = request(Command::ListFileRoots { after: None });
    let page = |roots, next| {
        Response::new(
            &listing,
            hub(HUB_ID),
            Reply::FileRoots { epoch, roots, next },
        )
    };
    assert!(
        page(vec![root.clone()], None)
            .validate_for(&listing)
            .is_ok()
    );
    for change_origin in [false, true] {
        let mut fields = root.fields().clone();
        if change_origin {
            fields.target.address.hub_id = hub(SECOND_HUB_ID);
        } else {
            fields.target.epoch = FileRootEpoch::new();
        }
        assert_eq!(
            page(vec![FileRootSummary::new(fields).unwrap()], None).validate_for(&listing),
            Err(PublicError::InvalidPayload)
        );
    }
    assert!(
        page(vec![root.clone(); MAX_FILE_ROOT_PAGE + PAGE_STEP], None)
            .validate_for(&listing)
            .is_err()
    );
    assert!(
        page(vec![root.clone(), root.clone()], None)
            .validate_for(&listing)
            .is_err()
    );
    assert!(
        page(vec![], Some(root.fields().target.address.root_id))
            .validate_for(&listing)
            .is_err()
    );
    assert!(
        page(vec![root.clone()], Some(FileRootId::new()))
            .validate_for(&listing)
            .is_err()
    );
    let after = request(Command::ListFileRoots {
        after: Some(root.fields().target.address.root_id),
    });
    assert!(
        Response::new(
            &after,
            hub(HUB_ID),
            Reply::FileRoots {
                epoch,
                roots: vec![root],
                next: None
            }
        )
        .validate_for(&after)
        .is_err()
    );
}

#[test]
fn read_reply_cannot_change_the_scope_or_requested_range() {
    let request = request(Command::ReadFile {
        request: file_read(),
    });
    let mut fields = file_reply().fields().clone();
    fields.target.epoch = FileRootEpoch::new();
    assert!(
        Response::new(
            &request,
            hub(HUB_ID),
            Reply::File {
                file: ReadFileResult::new(fields).unwrap()
            }
        )
        .validate_for(&request)
        .is_err()
    );
    let mut fields = file_reply().fields().clone();
    fields.data.pop();
    fields.eof = false;
    assert!(
        Response::new(
            &request,
            hub(HUB_ID),
            Reply::File {
                file: ReadFileResult::new(fields).unwrap()
            }
        )
        .validate_for(&request)
        .is_err()
    );
}
