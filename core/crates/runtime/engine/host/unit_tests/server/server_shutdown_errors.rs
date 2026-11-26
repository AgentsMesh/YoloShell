use crate::host::tests::*;
use super::*;

const HOST_SHUTDOWN_STAGES: &[HostShutdownStage] = &[
    HostShutdownStage::Service,
    HostShutdownStage::Ingress,
    HostShutdownStage::Maintenance,
];
const SERVICE_INGRESS_FAILURES: &[HostShutdownStage] =
    &[HostShutdownStage::Service, HostShutdownStage::Ingress];
const SERVICE_MAINTENANCE_FAILURES: &[HostShutdownStage] =
    &[HostShutdownStage::Service, HostShutdownStage::Maintenance];
const INGRESS_MAINTENANCE_FAILURES: &[HostShutdownStage] =
    &[HostShutdownStage::Ingress, HostShutdownStage::Maintenance];
const HOST_SHUTDOWN_NESTED_TEST_CONTEXT: &str = "nested host shutdown test context";

#[derive(Debug)]
struct HostShutdownTestError(HostShutdownStage);

impl std::fmt::Display for HostShutdownTestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(shutdown_test_error(self.0))
    }
}

impl std::error::Error for HostShutdownTestError {}

#[test]
fn single_shutdown_errors_keep_stage_context_and_native_source() {
    for &stage in HOST_SHUTDOWN_STAGES {
        let error = shutdown_result_for(&[stage]).unwrap_err();
        assert!(error.to_string().contains(stage.context()));
        assert_eq!(
            error
                .downcast_ref::<HostShutdownTestError>()
                .map(|error| error.0),
            Some(stage)
        );
        assert!(error.downcast_ref::<HostShutdownErrors>().is_none());
    }
}

#[test]
fn multiple_shutdown_errors_preserve_every_stage_and_source_chain() {
    for expected in [
        SERVICE_INGRESS_FAILURES,
        SERVICE_MAINTENANCE_FAILURES,
        INGRESS_MAINTENANCE_FAILURES,
        HOST_SHUTDOWN_STAGES,
    ] {
        let error = shutdown_result_for(expected).unwrap_err();
        let display = error.to_string();
        let aggregate = error.downcast_ref::<HostShutdownErrors>().unwrap();
        assert_eq!(aggregate.failures.len(), expected.len());
        let source = std::error::Error::source(aggregate).unwrap();
        assert_eq!(source.to_string(), HOST_SHUTDOWN_NESTED_TEST_CONTEXT);
        assert_eq!(
            source.source().map(ToString::to_string),
            Some(shutdown_test_error(expected[0]).to_owned())
        );
        for &stage in expected {
            assert!(display.contains(stage.context()));
            assert!(display.contains(shutdown_test_error(stage)));
            let failure = aggregate.failure(stage).unwrap();
            assert!(format!("{failure:#}").contains(HOST_SHUTDOWN_NESTED_TEST_CONTEXT));
            assert_eq!(
                failure
                    .downcast_ref::<HostShutdownTestError>()
                    .map(|error| error.0),
                Some(stage)
            );
        }
    }
}

fn shutdown_result_for(failed: &[HostShutdownStage]) -> Result<()> {
    host_shutdown_result(
        stage_result(HostShutdownStage::Service, failed),
        stage_result(HostShutdownStage::Ingress, failed),
        stage_result(HostShutdownStage::Maintenance, failed),
    )
}

fn stage_result(stage: HostShutdownStage, failed: &[HostShutdownStage]) -> Result<()> {
    if failed.contains(&stage) {
        Err(anyhow::Error::new(HostShutdownTestError(stage))
            .context(HOST_SHUTDOWN_NESTED_TEST_CONTEXT))
    } else {
        Ok(())
    }
}

fn shutdown_test_error(stage: HostShutdownStage) -> &'static str {
    match stage {
        HostShutdownStage::Service => HOST_SERVICE_TEST_ERROR,
        HostShutdownStage::Ingress => HOST_INGRESS_TEST_ERROR,
        HostShutdownStage::Maintenance => HOST_MAINTENANCE_TEST_ERROR,
    }
}
