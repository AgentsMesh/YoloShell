use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HostShutdownStage {
    Service,
    Ingress,
    Maintenance,
}

impl HostShutdownStage {
    pub(super) fn context(self) -> &'static str {
        match self {
            Self::Service => HOST_SERVICE_OUTCOME_CONTEXT,
            Self::Ingress => HOST_SHUTDOWN_INGRESS_CONTEXT,
            Self::Maintenance => HOST_SHUTDOWN_MAINTENANCE_CONTEXT,
        }
    }
}

#[derive(Debug)]
pub(super) struct HostShutdownFailure {
    pub(super) stage: HostShutdownStage,
    pub(super) error: anyhow::Error,
}

#[derive(Debug)]
pub(super) struct HostShutdownErrors {
    pub(super) failures: Vec<HostShutdownFailure>,
}

impl HostShutdownErrors {
    #[cfg(test)]
    pub(super) fn failure(&self, stage: HostShutdownStage) -> Option<&anyhow::Error> {
        self.failures
            .iter()
            .find(|failure| failure.stage == stage)
            .map(|failure| &failure.error)
    }
}

impl std::fmt::Display for HostShutdownErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{HOST_SHUTDOWN_MULTIPLE_CONTEXT}")?;
        for failure in &self.failures {
            write!(
                formatter,
                "; {}: {:#}",
                failure.stage.context(),
                failure.error
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for HostShutdownErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.failures.first().map(|failure| failure.error.as_ref())
    }
}

pub(super) fn host_shutdown_result(
    service: Result<()>,
    ingress: Result<()>,
    maintenance: Result<()>,
) -> Result<()> {
    let mut failures = Vec::new();
    for (stage, result) in [
        (HostShutdownStage::Service, service),
        (HostShutdownStage::Ingress, ingress),
        (HostShutdownStage::Maintenance, maintenance),
    ] {
        if let Err(error) = result {
            failures.push(HostShutdownFailure { stage, error });
        }
    }
    match failures.as_slice() {
        [] => Ok(()),
        [failure] => {
            let context = failure.stage.context();
            let failure = failures.pop().unwrap_or_else(|| unreachable!());
            Err(failure.error).context(context)
        }
        _ => Err(HostShutdownErrors { failures }.into()),
    }
}
