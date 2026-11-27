use super::*;
use crate::protocol::method;

pub(super) fn assert_session_requests(
    mut requests: api_matrix_support::MatrixRequests,
    terminal_id: Uuid,
    creation_capability: Uuid,
    creation_token: Uuid,
    create: &CreateTerminal,
) {
    requests.take_unique::<()>(method::HOST_STATUS);
    let listed = requests.take_unique::<ListSessionsRequest>(method::LIST_SESSIONS);
    assert!(listed.include_archived);
    assert!(!listed.require_complete);

    let get = requests.take_unique::<TerminalRef>(method::GET_SESSION);
    assert_eq!(get.terminal_id, terminal_id);
    let created = requests.take_unique::<CreateTerminal>(method::CREATE_SESSION);
    assert_eq!(&created, create);

    let terminate = requests.take_unique::<TerminalTerminateRequest>(method::TERMINATE_SESSION);
    assert_eq!(
        terminate.selection,
        TerminalLifecycleSelection::unconditional(terminal_id)
    );
    assert!(terminate.force);
    let archive = requests.take_unique::<TerminalLifecycleSelection>(method::ARCHIVE_SESSION);
    assert_eq!(archive, TerminalLifecycleSelection::unconditional(terminal_id));
    let discard = requests.take_unique::<TerminalDiscardRequest>(method::DISCARD_SESSION);
    assert_eq!(
        discard.selection,
        TerminalLifecycleSelection::unconditional(terminal_id)
    );
    assert!(discard.force);

    let cleanup_terminate =
        requests.take_unique::<CreationCleanupTerminateRequest>(method::CLEANUP_TERMINATE_SESSION);
    assert_eq!(
        cleanup_terminate,
        CreationCleanupTerminateRequest {
            terminal_id: terminal_id,
            creation_capability,
            force: true,
        }
    );
    let cleanup_archive =
        requests.take_unique::<CreationCleanupArchiveRequest>(method::CLEANUP_ARCHIVE_SESSION);
    assert_eq!(
        cleanup_archive,
        CreationCleanupArchiveRequest {
            terminal_id: terminal_id,
            creation_capability,
        }
    );
    let cleanup_discard =
        requests.take_unique::<CreationCleanupDiscardRequest>(method::CLEANUP_DISCARD_SESSION);
    assert_eq!(
        cleanup_discard,
        CreationCleanupDiscardRequest {
            terminal_id: terminal_id,
            creation_capability,
            force: true,
        }
    );

    let finalized = requests.take_unique::<FinalizeCreationRequest>(method::FINALIZE_CREATION);
    assert_eq!(
        finalized,
        FinalizeCreationRequest {
            creation_token,
            terminal_id: terminal_id,
            creation_capability,
        }
    );
    let resume = requests.take_unique::<ResumeRequest>(method::RESUME_SESSION_EXACT);
    assert_eq!(
        resume.selection,
        TerminalLifecycleSelection::unconditional(terminal_id)
    );
    assert!(resume.recreate);
    requests.take_unique::<()>(method::RECONCILE);
    requests.assert_empty();
}
