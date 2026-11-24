use super::publish_tests::support::published_count;
use super::tests::Fixture;
use super::*;
use serde_json::Value;
use std::fs;

const INJECTED_PUBLISH_FAILURE: &str = "injected broker generation publish failure";
const SINGLE_PUBLISHED_GENERATION: usize = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MatrixAction {
    Proceed,
    Fail(GenerationPublishStage),
    PublishWinner,
}

#[test]
fn one_hook_instance_covers_new_existing_competing_and_failure_paths() {
    let standard = Fixture::new();
    let first = run_stage(&standard, MatrixAction::Proceed).unwrap();
    let reused = run_stage(&standard, MatrixAction::Proceed).unwrap();
    assert_eq!(reused.config_path(), first.config_path());

    let optional = Fixture::new();
    remove_optional_material(&optional);
    run_stage(&optional, MatrixAction::Proceed).unwrap();

    let competing = Fixture::new();
    run_stage(&competing, MatrixAction::PublishWinner).unwrap();
    assert_eq!(
        published_count(&competing.paths),
        SINGLE_PUBLISHED_GENERATION
    );

    for stage in [
        GenerationPublishStage::SourceVerified,
        GenerationPublishStage::DestinationRenamed,
        GenerationPublishStage::DestinationCommitted,
        GenerationPublishStage::ExistingDurabilityConfirmed,
        GenerationPublishStage::ExistingOpened,
    ] {
        let fixture = Fixture::new();
        if matches!(
            stage,
            GenerationPublishStage::ExistingDurabilityConfirmed
                | GenerationPublishStage::ExistingOpened
        ) {
            run_stage(&fixture, MatrixAction::Proceed).unwrap();
        }
        assert!(run_stage(&fixture, MatrixAction::Fail(stage)).is_err());
        assert!(published_count(&fixture.paths) <= SINGLE_PUBLISHED_GENERATION);
    }
}

fn run_stage(fixture: &Fixture, action: MatrixAction) -> anyhow::Result<BrokerConfigGeneration> {
    stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
        match action {
            MatrixAction::Proceed => {}
            MatrixAction::Fail(expected) if stage == expected => {
                anyhow::bail!("{INJECTED_PUBLISH_FAILURE}: {stage:?}");
            }
            MatrixAction::PublishWinner if stage == GenerationPublishStage::SourceVerified => {
                stage_broker_config(&fixture.source, &fixture.paths)?;
            }
            MatrixAction::Fail(_) | MatrixAction::PublishWinner => {}
        }
        Ok(())
    })
}

fn remove_optional_material(fixture: &Fixture) {
    let mut value: Value = serde_json::from_slice(&fs::read(&fixture.source).unwrap()).unwrap();
    value.as_object_mut().unwrap().remove(ARTIFACT_CONFIG_FIELD);
    value[IDENTITIES_CONFIG_FIELD] = Value::Array(Vec::new());
    fs::write(&fixture.source, serde_json::to_vec(&value).unwrap()).unwrap();
}
