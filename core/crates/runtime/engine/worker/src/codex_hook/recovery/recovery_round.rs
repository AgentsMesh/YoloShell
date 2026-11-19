use super::recovery_route::CodexRecoveredLiveRoute;
use super::router::CodexRouterError;

#[must_use]
pub(super) struct CodexRecoveryRound {
    routes: Vec<CodexRecoveredLiveRoute>,
}

impl CodexRecoveryRound {
    pub(super) fn new(routes: Vec<CodexRecoveredLiveRoute>) -> Self {
        Self { routes }
    }

    pub(super) fn verify(&self) -> anyhow::Result<()> {
        for route in &self.routes {
            route.verify()?;
        }
        Ok(())
    }

    pub(super) fn accept(self) {
        for route in self.routes {
            route.accept();
        }
    }

    pub(super) fn rollback(self) -> Result<(), CodexRouterError> {
        let mut failure = None;
        for route in self.routes.into_iter().rev() {
            if let Err(error) = route.rollback() {
                failure.get_or_insert(error);
            }
        }
        match failure {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(super) fn reject(self, cause: anyhow::Error) -> anyhow::Error {
        match self.rollback() {
            Ok(()) => cause,
            Err(error) => anyhow::Error::new(error).context(cause),
        }
    }
}
