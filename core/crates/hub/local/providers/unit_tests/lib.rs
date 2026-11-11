use super::*;
use yolo_agent_contract::PROVIDER_CATALOG_CAPABILITY;
use yolo_hub_rpc_protocol::ProviderSummary;
use std::collections::BTreeSet;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn origin_service_methods_match_the_decorator_coverage() {
    let mut covered = BTreeSet::new();
    covered.extend(yolo_hub_local_terminal_history::ORIGIN_METHODS.iter().copied());
    covered.extend(ORIGIN_METHODS.iter().copied());
    let declared: BTreeSet<_> =
        yolo_hub_rpc_protocol::ORIGIN_SERVICE_METHODS.iter().copied().collect();
    assert_eq!(covered, declared);
}

fn provider(id: &str, name: &str) -> ProviderSummary {
    ProviderSummary::new(id, name, false).unwrap()
}

#[test]
fn provider_facts_are_projected_only_when_the_host_advertises_them() {
    let providers = vec![provider("alpha", "Alpha"), provider("beta", "Beta")];
    assert!(matches!(
        dispatcher::project(&providers, &[]),
        Err(PublicError::Unsupported)
    ));
    let values = dispatcher::project(
        &providers,
        &[
            PROVIDER_CATALOG_CAPABILITY.into(),
            "provider.beta".into(),
        ],
    )
    .unwrap();
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].provider_id, "alpha");
    assert!(!values[0].available);
    assert_eq!(values[1].provider_id, "beta");
    assert!(values[1].available);
}

#[test]
fn missing_provider_facts_are_not_proof_of_unavailability() {
    assert_eq!(dispatcher::provider_available(&[], "alpha"), None);
    assert_eq!(dispatcher::provider_available(&["other".into()], "alpha"), None);
    assert_eq!(
        dispatcher::provider_available(
            &[PROVIDER_CATALOG_CAPABILITY.into(), "provider.alpha".into()],
            "alpha",
        ),
        Some(true)
    );
    assert_eq!(
        dispatcher::provider_available(&[PROVIDER_CATALOG_CAPABILITY.into()], "alpha"),
        Some(false)
    );
}
