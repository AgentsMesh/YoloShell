use super::*;

#[test]
fn segment_geometry_rejects_coordinates_outside_llvm_bounds() {
    let invalid = code(ONE, ZERO, ONE, ONE, ONE);
    assert!(build_segments(&[invalid]).is_err());
}

#[test]
fn sparse_line_spans_cannot_exceed_the_protocol_allocation_budget() {
    let beyond_limit = yolo_coverage_protocol::MAXIMUM_INSTANCE_VECTOR_LENGTH + ONE;
    let segments = [
        counted(ONE, ONE, ONE, true, false),
        skipped(beyond_limit, ONE, false),
    ];
    assert_eq!(
        line_coverage(&segments).unwrap_err().to_string(),
        "coverage line vector exceeds the protocol limit"
    );
}

#[test]
fn nonentry_skipped_segments_do_not_invent_a_covered_line() {
    assert!(
        line_coverage(&[skipped(ONE, ONE, false)])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn empty_regions_and_lines_have_no_invented_coverage() {
    assert!(build_segments(&[]).unwrap().is_empty());
    assert!(line_coverage(&[]).unwrap().is_empty());
}

#[test]
fn identical_geometry_merges_only_matching_kinds_independent_of_input_order() {
    let regions = [
        region(RegionKind::Gap, TEN, ONE, ONE, THREE, ONE),
        code(TWO, ONE, ONE, THREE, ONE),
        region(RegionKind::Skipped, ZERO, ONE, ONE, THREE, ONE),
        region(RegionKind::Expansion, FIVE, ONE, ONE, THREE, ONE),
        code(THREE, ONE, ONE, THREE, ONE),
    ];
    let expected = [
        counted(ONE, ONE, FIVE, true, false),
        skipped(THREE, ONE, false),
    ];
    assert_eq!(build_segments(&regions).unwrap(), expected);
    assert_eq!(
        build_segments(&regions.into_iter().rev().collect::<Vec<_>>()).unwrap(),
        expected
    );
}

#[test]
fn equal_ends_restore_the_innermost_count_before_an_outer_region() {
    let regions = [
        code(ONE, ONE, ONE, TEN, ONE),
        code(TWO, TWO, ONE, FIVE, ONE),
        code(THREE, THREE, ONE, SIX, ONE),
        code(FOUR, FOUR, ONE, SIX, ONE),
    ];
    assert_eq!(
        build_segments(&regions).unwrap(),
        [
            counted(ONE, ONE, ONE, true, false),
            counted(TWO, ONE, TWO, true, false),
            counted(THREE, ONE, THREE, true, false),
            counted(FOUR, ONE, FOUR, true, false),
            counted(FIVE, ONE, FOUR, false, false),
            counted(SIX, ONE, ONE, false, false),
            skipped(TEN, ONE, false),
        ]
    );
    let extended = [regions.as_slice(), &[code(SIX, SIX, ONE, EIGHT, ONE)]].concat();
    let segments = build_segments(&extended).unwrap();
    assert!(segments.contains(&counted(FIVE, ONE, FOUR, false, false)));
    assert!(segments.contains(&counted(SIX, ONE, SIX, true, false)));
    assert!(segments.contains(&counted(EIGHT, ONE, ONE, false, false)));
    line_coverage(&segments).unwrap();
}

#[test]
fn coalesced_equal_count_gap_boundaries_preserve_line_counts() {
    let segments = build_segments(&[
        code(ONE, ONE, ONE, FIVE, ONE),
        region(RegionKind::Gap, ONE, TWO, ONE, THREE, ONE),
    ])
    .unwrap();
    assert_eq!(
        segments,
        [
            counted(ONE, ONE, ONE, true, false),
            counted(TWO, ONE, ONE, false, true),
            skipped(FIVE, ONE, false),
        ]
    );
    assert_eq!(
        line_coverage(&segments).unwrap(),
        lines(&[ONE, ONE, ONE, ONE, ONE])
    );
}

#[test]
fn point_regions_preserve_following_code_and_restore_active_counts() {
    let orphan = build_segments(&[
        code(TWO, ONE, ONE, ONE, ONE),
        code(THREE, TWO, ONE, FOUR, ONE),
    ])
    .unwrap();
    assert_eq!(orphan[0], counted(ONE, ONE, TWO, true, false));
    for kind in [RegionKind::Skipped, RegionKind::Gap] {
        let segments = build_segments(&[
            code(THREE, ONE, ONE, FIVE, ONE),
            region(kind, ZERO, THREE, ONE, THREE, ONE),
        ])
        .unwrap();
        assert!(segments.contains(&skipped(THREE, ONE, kind != RegionKind::Gap)));
        assert!(segments.contains(&counted(THREE, ONE, THREE, false, false)));
        line_coverage(&segments).unwrap();
    }
}

#[test]
fn every_region_coordinate_and_kind_is_validated() {
    assert_eq!(RegionKind::from_u64(ONE).unwrap(), RegionKind::Expansion);
    assert_eq!(RegionKind::from_u64(TWO).unwrap(), RegionKind::Skipped);
    for invalid in [
        code(ONE, ONE, ZERO, TWO, ONE),
        code(ONE, ONE, ONE, ZERO, ONE),
        code(ONE, ONE, ONE, TWO, ZERO),
        code(ONE, ONE, OUT_OF_RANGE, TWO, ONE),
        code(ONE, ONE, ONE, TWO, OUT_OF_RANGE),
        code(ONE, ONE, TWO, ONE, ONE),
    ] {
        assert!(build_segments(&[invalid]).is_err(), "{invalid:?}");
    }
    for invalid in [
        Segment {
            line: ZERO,
            ..counted(ONE, ONE, ONE, true, false)
        },
        Segment {
            column: OUT_OF_RANGE,
            ..counted(ONE, ONE, ONE, true, false)
        },
        Segment {
            is_gap_region: true,
            ..skipped(ONE, ONE, false)
        },
    ] {
        assert!(line_coverage(&[invalid]).is_err(), "{invalid:?}");
    }
    assert!(
        line_coverage(&[
            counted(ONE, ONE, ONE, true, false),
            skipped(ONE, ONE, false)
        ])
        .is_err()
    );
}
