use super::{
    fixtures::*,
    osc_support::{self, *},
};

#[test]
fn long_fragmented_osc_never_interprets_embedded_query_text_and_recovers_after_termination() {
    for termination in [BEL, CAN, SUB, ST] {
        let mut fixture = Fixture::new();
        fixture.quiet(THIRD_POSITION);
        let mut bytes = title(vte::MAX_OSC_RAW_BYTES + osc_support::STEP, &[]);
        bytes.extend_from_slice(EMBEDDED_QUERY_TEXT);
        bytes.extend_from_slice(termination);
        for chunk in bytes.chunks(CHUNK_BYTES) {
            assert!(fixture.live(chunk).is_empty());
        }
        assert_eq!(fixture.live(DSR), [STATUS.to_vec()]);
        assert_eq!(fixture.live(CPR), [THIRD_REPORT.to_vec()]);
        assert_eq!(fixture.live(DA), [ATTRIBUTES.to_vec()]);
    }
}

#[test]
fn oversized_osc_across_quiet_to_live_boundary_cannot_gain_query_authority() {
    let mut fixture = Fixture::new();
    let bytes = title(vte::MAX_OSC_RAW_BYTES + osc_support::STEP, &[]);
    fixture.quiet(&bytes[..CHUNK_BYTES]);
    for chunk in bytes[CHUNK_BYTES..].chunks(CHUNK_BYTES) {
        assert!(fixture.live(chunk).is_empty());
    }
    assert!(fixture.live(EMBEDDED_QUERY_TEXT).is_empty());
    assert!(fixture.live(BEL).is_empty());
    assert_eq!(fixture.live(DSR), [STATUS.to_vec()]);
    assert_eq!(fixture.live(CPR), [HOME_REPORT.to_vec()]);
}
