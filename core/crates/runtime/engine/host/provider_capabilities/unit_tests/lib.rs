use super::*;

#[test]
fn executable_probe_uses_the_host_path() {
    assert!(executable_available("/bin/sh"));
    assert!(!executable_available("yolo-missing-provider-probe-xyz"));
    assert!(executable_available("sh"));
}

#[test]
fn provider_capabilities_are_prefixed_sorted_and_unique() {
    let values = provider_capabilities();
    let mut sorted = values.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(values, sorted);
    assert!(values
        .iter()
        .all(|value| value.starts_with(PROVIDER_CAPABILITY_PREFIX)));
    assert!(!values.iter().any(|value| value == PROVIDER_CAPABILITY_PREFIX));
}
