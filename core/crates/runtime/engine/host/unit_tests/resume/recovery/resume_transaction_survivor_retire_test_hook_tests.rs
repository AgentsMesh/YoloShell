use super::*;

#[test]
fn boundary_names_round_trip_without_aliases() {
    for boundary in Boundary::ALL {
        assert_eq!(Boundary::parse(boundary.as_str()).unwrap(), boundary);
    }
    assert!(Boundary::parse(INVALID_BOUNDARY_ALIAS).is_err());
}
