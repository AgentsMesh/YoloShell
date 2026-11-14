use anyhow::Context as _;

pub(super) const BROKER_MAINTENANCE_ERROR_CONTEXT: &str = "broker maintenance shutdown failed";
const BROKER_NAMESPACE_ERROR_CONTEXT: &str = "broker namespace shutdown failed";

#[derive(Debug)]
pub struct BrokerShutdownErrors {
    namespace: anyhow::Error,
    maintenance: anyhow::Error,
}

impl BrokerShutdownErrors {
    pub fn namespace_error(&self) -> &anyhow::Error {
        &self.namespace
    }

    pub fn maintenance_error(&self) -> &anyhow::Error {
        &self.maintenance
    }
}

impl std::fmt::Display for BrokerShutdownErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{BROKER_NAMESPACE_ERROR_CONTEXT}: {:#}; {BROKER_MAINTENANCE_ERROR_CONTEXT}: {:#}",
            self.namespace, self.maintenance
        )
    }
}

impl std::error::Error for BrokerShutdownErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.namespace.as_ref())
    }
}

pub(super) enum BrokerShutdownCause {
    Rotation,
    Retirement,
    External,
    NamespaceLost(anyhow::Error),
}

fn namespace_shutdown(result: Result<std::convert::Infallible>) -> BrokerShutdownCause {
    match result {
        Ok(value) => match value {},
        Err(error) => BrokerShutdownCause::NamespaceLost(error),
    }
}

pub(super) fn shutdown_result(cause: BrokerShutdownCause, maintenance: Result<()>) -> Result<()> {
    let cause = match cause {
        BrokerShutdownCause::NamespaceLost(error) => Err(error),
        BrokerShutdownCause::Rotation
        | BrokerShutdownCause::Retirement
        | BrokerShutdownCause::External => Ok(()),
    };
    match (cause, maintenance) {
        (Err(namespace), Err(maintenance)) => Err(BrokerShutdownErrors {
            namespace,
            maintenance,
        }
        .into()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error).context(BROKER_MAINTENANCE_ERROR_CONTEXT),
        (Ok(()), Ok(())) => Ok(()),
    }
}
