use super::publish_tests::support::{pending_count, published_count};
use super::tests::Fixture;
use super::*;

const FAILED_PUBLICATIONS: usize = MAX_CONCURRENT_GENERATION_STAGES * 4;

#[test]
fn repeated_failed_publications_reuse_a_bounded_slot() {
    let fixture = Fixture::new();
    for _ in 0..FAILED_PUBLICATIONS {
        let result = stage_broker_config_with_hook(&fixture.source, &fixture.paths, |stage| {
            if stage == GenerationPublishStage::SourceVerified {
                anyhow::bail!("injected publish failure");
            }
            Ok(())
        });
        assert!(result.is_err());
    }
    assert_eq!(pending_count(&fixture.paths), 1);
    assert_eq!(published_count(&fixture.paths), 0);
}
