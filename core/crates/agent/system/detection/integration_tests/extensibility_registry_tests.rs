use super::*;

#[test]
fn provider_environment_cannot_override_runtime_identity() {
    let environment = [(FUTURE_RESERVED_ENV.to_owned(), String::new())].into();
    assert!(validate_launch_environment(&environment).is_err());
}

#[test]
fn registry_rejects_empty_and_duplicate_provider_kinds() {
    let empty = AgentRegistry::from_adapters(vec![Box::new(FutureAdapter {
        kind: AgentKind::from_name(String::new()),
    })])
    .err()
    .unwrap();
    assert!(
        empty
            .to_string()
            .contains(constants::validation::REGISTRY_KIND_EMPTY)
    );

    let duplicate =
        AgentRegistry::from_adapters(vec![Box::new(future_adapter()), Box::new(future_adapter())])
            .err()
            .unwrap();
    assert!(
        duplicate
            .to_string()
            .contains(constants::validation::REGISTRY_KIND_DUPLICATE)
    );
}
