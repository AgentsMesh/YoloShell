use crate::segments::Segment;
use std::fmt;
use yolo_coverage_protocol::MAXIMUM_INSTANCE_VECTOR_LENGTH;

const MIN_COORDINATE: u64 = 1;
const MAX_COORDINATE: u64 = u32::MAX as u64;
const ZERO_COUNT: u64 = 0;
const FIRST_INDEX: usize = 0;
const NEXT_INDEX: usize = 1;
const MULTIPLE_REGIONS: usize = 2;
const CODE_KIND: u64 = 0;
const EXPANSION_KIND: u64 = 1;
const SKIPPED_KIND: u64 = 2;
const GAP_KIND: u64 = 3;
const INVALID_KIND: &str = "invalid LLVM region kind";
const INVALID_SEGMENT: &str = "invalid LLVM coverage segment";
const COORDINATE_OVERFLOW: &str = "LLVM line coordinate overflow";
const METRIC_VECTOR_LIMIT: &str = "coverage line vector exceeds the protocol limit";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RegionKind {
    Code,
    Expansion,
    Skipped,
    Gap,
}

impl RegionKind {
    pub fn from_u64(value: u64) -> Result<Self, SegmentError> {
        match value {
            CODE_KIND => Ok(Self::Code),
            EXPANSION_KIND => Ok(Self::Expansion),
            SKIPPED_KIND => Ok(Self::Skipped),
            GAP_KIND => Ok(Self::Gap),
            _ => Err(SegmentError::new(INVALID_KIND)),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Region {
    pub start_line: u64,
    pub start_column: u64,
    pub end_line: u64,
    pub end_column: u64,
    pub count: u64,
    pub kind: RegionKind,
}

impl Region {
    pub(crate) fn start(self) -> (u64, u64) {
        (self.start_line, self.start_column)
    }

    pub(crate) fn end(self) -> (u64, u64) {
        (self.end_line, self.end_column)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SegmentError(&'static str);

impl SegmentError {
    pub(crate) fn new(message: &'static str) -> Self {
        Self(message)
    }
}

impl fmt::Display for SegmentError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineCoverage {
    pub line: u64,
    pub count: u64,
}

pub fn line_coverage(segments: &[Segment]) -> Result<Vec<LineCoverage>, SegmentError> {
    validate_segments(segments)?;
    let Some(first) = segments.first() else {
        return Ok(Vec::new());
    };
    let last_line = segments.last().expect("non-empty segment set").line;
    let span = last_line
        .checked_sub(first.line)
        .and_then(|value| value.checked_add(MIN_COORDINATE))
        .ok_or_else(|| SegmentError::new(COORDINATE_OVERFLOW))?;
    if span > MAXIMUM_INSTANCE_VECTOR_LENGTH {
        return Err(SegmentError::new(METRIC_VECTOR_LIMIT));
    }
    let mut output = Vec::new();
    let (mut line, mut next, mut wrapped) = (first.line, FIRST_INDEX, None);
    loop {
        let start = next;
        while next < segments.len() && segments[next].line == line {
            next += NEXT_INDEX;
        }
        let current = &segments[start..next];
        let starts = current
            .iter()
            .filter(|segment| starts_region(segment))
            .take(MULTIPLE_REGIONS)
            .count();
        let starts_skipped = current
            .first()
            .is_some_and(|segment| !segment.has_count && segment.is_region_entry);
        let mapped = (!starts_skipped
            && (wrapped.is_some_and(|segment: &Segment| segment.has_count)
                || starts > FIRST_INDEX))
            || current
                .iter()
                .any(|segment| segment.is_region_entry && segment.has_count);
        if mapped {
            let mut count = wrapped.map_or(ZERO_COUNT, |segment: &Segment| segment.count);
            for segment in current.iter().filter(|segment| starts_region(segment)) {
                count = count.max(segment.count);
            }
            output.push(LineCoverage { line, count });
        }
        if let Some(segment) = current.last() {
            wrapped = Some(segment);
        }
        if line == last_line {
            break;
        }
        line = line
            .checked_add(MIN_COORDINATE)
            .ok_or_else(|| SegmentError::new(COORDINATE_OVERFLOW))?;
    }
    Ok(output)
}

fn starts_region(segment: &&Segment) -> bool {
    !segment.is_gap_region && segment.has_count && segment.is_region_entry
}

fn validate_segments(segments: &[Segment]) -> Result<(), SegmentError> {
    for (index, segment) in segments.iter().enumerate() {
        let position = (segment.line, segment.column);
        let invalid_position = [position.0, position.1]
            .iter()
            .any(|value| !(MIN_COORDINATE..=MAX_COORDINATE).contains(value));
        let invalid_count =
            !segment.has_count && (segment.count != ZERO_COUNT || segment.is_gap_region);
        let invalid_order = index > FIRST_INDEX && {
            let previous = &segments[index - NEXT_INDEX];
            (previous.line, previous.column) > position
                || ((previous.line, previous.column) == position && previous.has_count)
        };
        if invalid_position || invalid_count || invalid_order {
            return Err(SegmentError::new(INVALID_SEGMENT));
        }
    }
    Ok(())
}
