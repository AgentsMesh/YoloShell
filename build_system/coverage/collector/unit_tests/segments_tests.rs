use crate::segments::{LineCoverage, Region, RegionKind, Segment, build_segments, line_coverage};

#[path = "segment_boundary_tests.rs"]
mod boundary_tests;

const ZERO: u64 = 0;
const ONE: u64 = 1;
const TWO: u64 = 2;
const THREE: u64 = 3;
const FOUR: u64 = 4;
const FIVE: u64 = 5;
const SIX: u64 = 6;
const SEVEN: u64 = 7;
const EIGHT: u64 = 8;
const TEN: u64 = 10;
const THIRTY: u64 = 30;
const CODE_KIND_VALUE: u64 = 0;
const GAP_KIND_VALUE: u64 = 3;
const UNKNOWN_KIND_VALUE: u64 = 99;
const OUT_OF_RANGE: u64 = u32::MAX as u64 + ONE;
const TWO_HITS: usize = 2;
const THREE_HITS: usize = 3;

#[test]
fn llvm_a_through_e_nested_regions_restore_enclosing_counts() {
    let regions = [
        code(ONE, ONE, ONE, EIGHT, EIGHT),
        code(TWO, TWO, TWO, FIVE, FIVE),
        code(THREE, THREE, THREE, FOUR, FOUR),
        code(FOUR, SIX, SIX, SEVEN, SEVEN),
    ];
    let actual = build_segments(&regions).unwrap();
    let expected = [
        counted(ONE, ONE, ONE, true, false),
        counted(TWO, TWO, TWO, true, false),
        counted(THREE, THREE, THREE, true, false),
        counted(FOUR, FOUR, TWO, false, false),
        counted(FIVE, FIVE, ONE, false, false),
        counted(SIX, SIX, FOUR, true, false),
        counted(SEVEN, SEVEN, ONE, false, false),
        skipped(EIGHT, EIGHT, false),
    ];
    assert_eq!(actual, expected);
    assert_eq!(
        line_coverage(&actual).unwrap(),
        lines(&[ONE, TWO, THREE, THREE, TWO, FOUR, FOUR, ONE])
    );
}

#[test]
fn gap_regions_preserve_llvm_wrapped_line_semantics() {
    let regions = [
        code(THIRTY, ONE, ONE, FIVE, ONE),
        region(RegionKind::Gap, TEN, TWO, ONE, THREE, ONE),
    ];
    let segments = build_segments(&regions).unwrap();
    assert_eq!(
        segments,
        [
            counted(ONE, ONE, THIRTY, true, false),
            counted(TWO, ONE, TEN, false, true),
            counted(THREE, ONE, THIRTY, false, false),
            skipped(FIVE, ONE, false),
        ]
    );
    assert_eq!(
        line_coverage(&segments).unwrap(),
        lines(&[THIRTY, THIRTY, TEN, THIRTY, THIRTY])
    );
}

#[test]
fn skipped_region_entry_makes_its_line_unmapped() {
    let regions = [
        code(ONE, ONE, ONE, FIVE, FIVE),
        region(RegionKind::Skipped, ZERO, FIVE, ONE, FIVE, ONE),
    ];
    let segments = build_segments(&regions).unwrap();
    assert!(segments.contains(&skipped(FIVE, ONE, true)));
    assert_eq!(
        line_coverage(&segments).unwrap(),
        lines(&[ONE, ONE, ONE, ONE])
    );
}

#[test]
fn zero_length_regions_follow_llvm_predecessor_rules() {
    let consecutive = [code(ONE, ONE, ONE, ONE, ONE), code(TWO, ONE, ONE, ONE, ONE)];
    assert_eq!(
        build_segments(&consecutive).unwrap(),
        [skipped(ONE, ONE, true)]
    );

    let sandwiched = [
        code(TWO, ONE, ONE, FOUR, FOUR),
        code(ONE, THREE, ONE, THREE, ONE),
        code(ONE, THREE, TWO, THREE, THREE),
    ];
    let segments = build_segments(&sandwiched).unwrap();
    assert!(segments.contains(&counted(THREE, ONE, TWO, true, false)));
    assert!(segments.contains(&counted(THREE, THREE, TWO, false, false)));
}

#[test]
fn complementary_generic_instances_remain_independent() {
    let instance_a = [
        code(ONE, ONE, ONE, FOUR, FOUR),
        code(ZERO, TWO, ONE, THREE, ONE),
    ];
    let instance_b = [
        code(ZERO, ONE, ONE, FOUR, FOUR),
        code(ONE, TWO, ONE, THREE, ONE),
    ];
    let lines_a = line_coverage(&build_segments(&instance_a).unwrap()).unwrap();
    let lines_b = line_coverage(&build_segments(&instance_b).unwrap()).unwrap();
    assert_eq!(lines_a, lines(&[ONE, ONE, ZERO, ONE]));
    assert_eq!(lines_b, lines(&[ZERO, ONE, ONE, ZERO]));
    assert_eq!((hits(&lines_a), hits(&lines_b)), (THREE_HITS, TWO_HITS));
}

#[test]
fn malformed_kinds_coordinates_counts_and_segments_fail_closed() {
    assert_eq!(
        RegionKind::from_u64(CODE_KIND_VALUE).unwrap(),
        RegionKind::Code
    );
    assert_eq!(
        RegionKind::from_u64(GAP_KIND_VALUE).unwrap(),
        RegionKind::Gap
    );
    assert!(RegionKind::from_u64(UNKNOWN_KIND_VALUE).is_err());
    for invalid in [
        code(ONE, ZERO, ONE, ONE, ONE),
        code(ONE, TWO, ONE, ONE, ONE),
        code(ONE, OUT_OF_RANGE, ONE, OUT_OF_RANGE, TWO),
    ] {
        assert!(build_segments(&[invalid]).is_err());
    }
    let overflowing = [
        code(u64::MAX, ONE, ONE, TWO, TWO),
        code(ONE, ONE, ONE, TWO, TWO),
    ];
    assert!(build_segments(&overflowing).is_err());
    let bad_count = Segment {
        count: ONE,
        ..skipped(ONE, ONE, false)
    };
    let reversed = [
        counted(TWO, ONE, ONE, true, false),
        counted(ONE, ONE, ONE, true, false),
    ];
    assert!(line_coverage(&[bad_count]).is_err());
    assert!(line_coverage(&reversed).is_err());
}

fn code(count: u64, sl: u64, sc: u64, el: u64, ec: u64) -> Region {
    region(RegionKind::Code, count, sl, sc, el, ec)
}

#[rustfmt::skip]
fn region(kind: RegionKind, count: u64, sl: u64, sc: u64, el: u64, ec: u64) -> Region {
    Region {
        start_line: sl, start_column: sc, end_line: el,
        end_column: ec, count, kind,
    }
}

#[rustfmt::skip]
fn counted(line: u64, column: u64, count: u64, entry: bool, gap: bool) -> Segment {
    Segment {
        line, column, count, has_count: true,
        is_region_entry: entry, is_gap_region: gap,
    }
}

#[rustfmt::skip]
fn skipped(line: u64, column: u64, entry: bool) -> Segment {
    Segment {
        line, column, count: ZERO, has_count: false,
        is_region_entry: entry, is_gap_region: false,
    }
}

fn lines(counts: &[u64]) -> Vec<LineCoverage> {
    (ONE..)
        .zip(counts.iter().copied())
        .map(|(line, count)| LineCoverage { line, count })
        .collect()
}

fn hits(lines: &[LineCoverage]) -> usize {
    lines.iter().filter(|line| line.count > ZERO).count()
}
