use super::*;

const DEADLINE_BUDGET: Duration = Duration::from_secs(1);
const INVALID_ENCODING: &str = "not-a-deadline";
const TIGHT_BRACKET_NANOSECONDS: u128 = 1;

#[test]
fn monotonic_deadline_round_trip_never_extends_parent_budget() {
    let parent = tokio::time::Instant::now() + DEADLINE_BUDGET;
    let encoded = BrokerLaunchDeadline::capture(parent).unwrap();
    let child = encoded.to_instant().unwrap();

    assert!(child <= parent);
    assert!(child > tokio::time::Instant::now());
    assert_eq!(
        encoded.to_string().parse::<BrokerLaunchDeadline>().unwrap(),
        encoded
    );
}

#[test]
fn monotonic_anchor_rejects_injected_preemption_and_uses_the_tightest_bracket() {
    let excessive = MONOTONIC_ANCHOR_MAX_SKEW.as_nanos() + TIGHT_BRACKET_NANOSECONDS;
    let mut samples = (0..MONOTONIC_ANCHOR_SAMPLE_ATTEMPTS)
        .flat_map(|attempt| {
            let before = attempt as u128 * excessive;
            let skew = if attempt + 1 == MONOTONIC_ANCHOR_SAMPLE_ATTEMPTS {
                TIGHT_BRACKET_NANOSECONDS
            } else {
                excessive
            };
            [before, before + skew]
        })
        .into_iter();
    let anchor = select_monotonic_anchor(|| Ok(samples.next().unwrap())).unwrap();

    assert_eq!(
        anchor.after_nanoseconds - anchor.before_nanoseconds,
        TIGHT_BRACKET_NANOSECONDS
    );
}

#[test]
fn expired_and_malformed_deadlines_fail_closed() {
    let expired = BrokerLaunchDeadline {
        boot_sha256: current_boot_sha256().unwrap(),
        monotonic_nanoseconds: u128::default(),
    };

    assert!(expired.to_instant().is_err());
    assert!(INVALID_ENCODING.parse::<BrokerLaunchDeadline>().is_err());
    assert!(BrokerLaunchDeadline::capture(tokio::time::Instant::now()).is_err());
}
