use crate::arguments::{Arguments, Command, WorkspaceCommand, WorktreeCommand};
use clap::error::ErrorKind;
use clap::Parser;

mod constants;

fn workspace_update(extra: &[&str]) -> crate::arguments::UpdateArgs {
    let mut arguments = vec![constants::PROGRAM, constants::WORKSPACE, constants::UPDATE, "demo"];
    arguments.extend_from_slice(extra);
    let parsed = Arguments::try_parse_from(arguments).unwrap();
    let Command::Workspace(options) = parsed.command else {
        panic!("expected workspace")
    };
    let WorkspaceCommand::Update(update) = options.command else {
        panic!("expected workspace update")
    };
    update
}

#[test]
fn workspace_update_revision_is_optional_and_still_validateable() {
    assert_eq!(
        workspace_update(&[constants::NAME, "renamed"]).expected_revision,
        None
    );
    assert_eq!(
        workspace_update(&[
            constants::NAME,
            "renamed",
            constants::EXPECTED_REVISION,
            "7"
        ])
        .expected_revision,
        Some(7)
    );
}

#[test]
fn worktree_create_requires_the_repository_trust_acknowledgement() {
    let missing = Arguments::try_parse_from([
        constants::PROGRAM,
        constants::WORKTREE,
        constants::CREATE,
        constants::PARENT,
        constants::NAME,
        constants::WORKTREE_NAME,
    ])
    .unwrap_err();
    assert_eq!(missing.kind(), ErrorKind::MissingRequiredArgument);

    let parsed = Arguments::try_parse_from([
        constants::PROGRAM,
        constants::WORKTREE,
        constants::CREATE,
        constants::PARENT,
        constants::NAME,
        constants::WORKTREE_NAME,
        constants::TRUST_REPOSITORY,
    ])
    .unwrap();
    let Command::Worktree(options) = parsed.command else {
        panic!("expected worktree")
    };
    let WorktreeCommand::Create(create) = options.command else {
        panic!("expected worktree create")
    };
    assert!(create.trust_repository);
}
