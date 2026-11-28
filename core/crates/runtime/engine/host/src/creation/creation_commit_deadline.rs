use super::*;

#[derive(Debug)]
pub(in crate::host) struct CreationPublicationDeadlineExpired;

#[derive(Debug)]
pub(in crate::host) struct CreationPublicationCommitInFlight;

impl std::fmt::Display for CreationPublicationDeadlineExpired {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(CREATION_PUBLICATION_DEADLINE_EXPIRED)
    }
}

impl std::error::Error for CreationPublicationDeadlineExpired {}

impl std::fmt::Display for CreationPublicationCommitInFlight {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(CREATION_PUBLICATION_COMMIT_IN_FLIGHT)
    }
}

impl std::error::Error for CreationPublicationCommitInFlight {}

pub(crate) fn require_creation_commit_budget(deadline: HostOperationDeadline) -> Result<()> {
    if deadline.is_commit_elapsed() {
        Err(anyhow::Error::new(CreationPublicationDeadlineExpired))
    } else {
        Ok(())
    }
}

pub(crate) fn require_creation_publication_budget(deadline: HostOperationDeadline) -> Result<()> {
    if deadline.is_elapsed() {
        Err(anyhow::Error::new(CreationPublicationDeadlineExpired))
    } else {
        Ok(())
    }
}

pub(in crate::host) fn creation_publication_error(error: anyhow::Error) -> ErrorBody {
    if error.is::<CreationPublicationDeadlineExpired>()
        || error.is::<CreationPublicationCommitInFlight>()
    {
        creation_launch_uncertain()
    } else {
        internal(error)
    }
}

include!("creation_catalog_commit.rs");
