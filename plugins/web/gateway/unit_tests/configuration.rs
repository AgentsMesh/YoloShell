use super::*;

fn config(
    origins: Vec<WebOrigin>,
    lifetime: Duration,
    capacity: usize,
) -> anyhow::Result<GatewayConfig> {
    GatewayConfig::new(
        "127.0.0.1:0".parse().unwrap(),
        origins,
        AccessToken::from_hex(TOKEN).unwrap(),
        lifetime,
        capacity,
    )
}

#[test]
fn empty_and_oversized_origin_sets_cannot_open_a_gateway() {
    assert!(
        config(
            Vec::new(),
            DEFAULT_SESSION_LIFETIME,
            DEFAULT_MAX_CONNECTIONS
        )
        .is_err()
    );
    let origins = (0..=MAX_ORIGINS)
        .map(|index| WebOrigin::parse(format!("https://frontend-{index}.test")).unwrap())
        .collect::<Vec<_>>();
    assert!(
        config(
            origins.clone(),
            DEFAULT_SESSION_LIFETIME,
            DEFAULT_MAX_CONNECTIONS
        )
        .is_err()
    );
    assert!(
        config(
            origins[..MAX_ORIGINS].to_vec(),
            DEFAULT_SESSION_LIFETIME,
            DEFAULT_MAX_CONNECTIONS
        )
        .is_ok()
    );
}

#[test]
fn resource_limits_accept_the_boundary_and_reject_excess() {
    let origins = vec![WebOrigin::parse("https://example.test").unwrap()];
    assert!(config(origins.clone(), MAX_SESSION_LIFETIME, MAX_CONNECTIONS).is_ok());
    assert!(config(origins.clone(), MAX_SESSION_LIFETIME, MAX_CONNECTIONS + 1).is_err());
    assert!(
        config(
            origins,
            MAX_SESSION_LIFETIME.saturating_mul(2),
            MAX_CONNECTIONS
        )
        .is_err()
    );
}
