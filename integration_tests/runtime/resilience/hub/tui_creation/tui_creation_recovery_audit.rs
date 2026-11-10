use super::{constants::*, journal};
use std::{path::Path, sync::Mutex};
use tokio::sync::Notify;
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_hub_terminal_protocol::CreateShellResult;
use yolo_tui_creation_journal::CreationIntent;
#[path = "tui_creation_recovery/test_support/layout_audit.rs"]
mod layout;

#[derive(Clone, Copy, Debug)]
pub enum Failure {
    Accept,
    Connect,
    ReadRequest,
    WriteRequest,
    ReadReply,
    InvalidReply,
    JournalBeforeCreate,
    RepeatedCreate,
    QueryIdentity,
    FinalizeIdentity,
    UnexpectedWriter,
    UnexpectedMutation,
    LayoutIdentity,
    UnexpectedCreateReply,
    HeldRequest,
    Task,
    Capacity,
}

#[derive(Default)]
pub struct Shared {
    pub audit: Mutex<Audit>,
    pub changed: Notify,
}

#[derive(Default)]
pub struct Audit {
    pub failure: Option<Failure>,
    pub intent: Option<CreationIntent>,
    pub creation: Option<CreateShellResult>,
    pub creates: usize,
    pub queries: usize,
    pub finalizes: usize,
    pub finalized: bool,
    pub held_closed: bool,
}

impl Shared {
    pub fn fail(&self, failure: Failure) {
        self.audit.lock().unwrap().failure.get_or_insert(failure);
        self.changed.notify_one();
    }
}

impl Audit {
    pub fn observe(&mut self, request: &Request, workspace: &Path) -> Result<bool, Failure> {
        match &request.fields().command {
            Command::Create { request: creation } => {
                self.creates += COUNT_STEP;
                if self.creates != ONLY_CREATION {
                    return Err(Failure::RepeatedCreate);
                }
                let intent = CreationIntent::new(request.fields().destination, creation.clone())
                    .map_err(|_| Failure::JournalBeforeCreate)?;
                if !journal::before_create(workspace, &intent) {
                    return Err(Failure::JournalBeforeCreate);
                }
                self.intent = Some(intent);
                Ok(true)
            }
            Command::QueryCreation { request: query } => {
                let expected = self.intent.as_ref().ok_or(Failure::QueryIdentity)?;
                if expected.origin != request.fields().destination
                    || query.creation() != &expected.request
                {
                    return Err(Failure::QueryIdentity);
                }
                self.queries += COUNT_STEP;
                Ok(false)
            }
            Command::Finalize {
                creation_token,
                creation,
            } => {
                let expected = self.intent.as_ref().ok_or(Failure::FinalizeIdentity)?;
                if self.creation != Some(*creation)
                    || expected.origin != request.fields().destination
                    || *creation_token != expected.request.fields().creation_token
                    || self.queries == usize::default()
                {
                    return Err(Failure::FinalizeIdentity);
                }
                self.finalizes += COUNT_STEP;
                Ok(false)
            }
            Command::UpdateLayout { .. } => {
                layout::request(request, workspace).map_err(|_| Failure::LayoutIdentity)?;
                Ok(false)
            }
            Command::Lease { .. }
            | Command::Renew { .. }
            | Command::Input { .. }
            | Command::Resize { .. }
            | Command::AgentControl { .. } => Err(Failure::UnexpectedWriter),
            command if command.mutates() => Err(Failure::UnexpectedMutation),
            _ => Ok(false),
        }
    }

    pub fn reply(
        &mut self,
        request: &Request,
        response: &Response,
        held: bool,
    ) -> Result<(), Failure> {
        if held {
            let Reply::Created { creation } = &response.reply else {
                return Err(Failure::UnexpectedCreateReply);
            };
            self.creation = Some(*creation);
        }
        if matches!(request.fields().command, Command::Finalize { .. }) {
            if !matches!(response.reply, Reply::Completed {}) {
                return Err(Failure::FinalizeIdentity);
            }
            self.finalized = true;
        }
        layout::reply(request, &response.reply).map_err(|_| Failure::LayoutIdentity)?;
        Ok(())
    }

    pub fn verify(&self) {
        assert!(
            self.failure.is_none(),
            "creation recovery proxy failed: {:?}",
            self.failure
        );
        assert_eq!(self.creates, ONLY_CREATION);
        assert!(self.queries >= ONLY_CREATION);
        assert!(self.finalizes >= ONLY_CREATION && self.finalized);
        assert!(self.held_closed);
    }
}
