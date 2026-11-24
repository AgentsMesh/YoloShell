//! Durable workspace metadata. File-system truth remains with the Host; this
//! protocol carries names, roots, labels, revisions and explicit capabilities.

mod constants;
mod request;
mod types;
mod validation;
mod worktree;

pub use constants::*;
pub use request::{
    CreateWorkspaceFields, CreateWorkspaceRequest, GetWorkspaceFields, GetWorkspaceRequest,
    ListWorkspacesFields, ListWorkspacesRequest, ListWorkspacesResult, UpdateWorkspaceFields,
    UpdateWorkspaceRequest, WorkspaceResult,
};
pub use types::{
    GitCapability, MembershipChange, WorkspaceAddress, WorkspaceCapabilities, WorkspaceKind,
    WorkspaceSelector, WorkspaceSummary, WorktreeCapability, address,
};
pub use validation::{validate_labels, validate_membership, validate_name, validate_root};
pub use worktree::*;
