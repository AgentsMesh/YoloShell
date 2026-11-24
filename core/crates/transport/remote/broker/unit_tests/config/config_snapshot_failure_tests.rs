use super::tests::Fixture;
use super::*;
use crate::remote_broker::BROKER_READY_TIMEOUT;

const EXISTING_DURABILITY_HOOK_FAILURE: &str = "injected existing durability failure";
const SOURCE_VERIFIED_FAILURE: &str = "injected source verification failure";
const DESTINATION_COMMITTED_FAILURE: &str = "injected destination commit failure";

#[tokio::test]
async fn public_guarded_stage_publishes_in_the_retained_namespace() {
    let fixture = Fixture::new();
    let guard = BrokerReconcileGuard::acquire(
        &fixture.paths,
        tokio::time::Instant::now() + BROKER_READY_TIMEOUT,
    )
    .await
    .unwrap();

    let generation = stage_broker_config_guarded(&fixture.source, &fixture.paths, &guard).unwrap();

    assert!(generation.config_path().is_file());
    guard.verify().unwrap();
}

#[test]
fn existing_generation_propagates_durability_hook_failure_without_replacement() {
    let fixture = Fixture::new();
    let expected = stage_broker_config(&fixture.source, &fixture.paths).unwrap();

    let error = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::ExistingDurabilityConfirmed {
            anyhow::bail!(EXISTING_DURABILITY_HOOK_FAILURE);
        }
        Ok(())
    })
    .unwrap_err();

    assert!(error.to_string().contains(EXISTING_DURABILITY_HOOK_FAILURE));
    let recovered = stage_broker_config(&fixture.source, &fixture.paths).unwrap();
    assert_eq!(recovered.config_path(), expected.config_path());
}

#[test]
fn publication_hook_failures_leave_a_reusable_generation_namespace() {
    let fixture = Fixture::new();
    let source_error = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        if stage == GenerationPublishStage::SourceVerified {
            anyhow::bail!(SOURCE_VERIFIED_FAILURE);
        }
        Ok(())
    })
    .unwrap_err();
    assert!(source_error.to_string().contains(SOURCE_VERIFIED_FAILURE));
    let generation = stage_broker_config(&fixture.source, &fixture.paths).unwrap();

    let committed_fixture = Fixture::new();
    let committed_error = stage_broker_config_with_hook(
        &committed_fixture.source,
        &committed_fixture.paths,
        |stage| {
            if stage == GenerationPublishStage::DestinationCommitted {
                anyhow::bail!(DESTINATION_COMMITTED_FAILURE);
            }
            Ok(())
        },
    )
    .unwrap_err();
    assert!(
        committed_error
            .to_string()
            .contains(DESTINATION_COMMITTED_FAILURE)
    );
    let committed =
        stage_broker_config(&committed_fixture.source, &committed_fixture.paths).unwrap();

    assert!(generation.config_path().is_file());
    assert!(committed.config_path().is_file());
}
