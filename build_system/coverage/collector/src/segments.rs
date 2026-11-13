#[cfg(test)]
pub use crate::segment_lines::LineCoverage;
pub use crate::segment_lines::{Region, RegionKind, SegmentError, line_coverage};

type Position = (u64, u64);
const MIN_COORDINATE: u64 = 1;
const MAX_COORDINATE: u64 = u32::MAX as u64;
const ZERO_COUNT: u64 = 0;
const FIRST_INDEX: usize = 0;
const NEXT_INDEX: usize = 1;
const INVALID_REGION: &str = "invalid LLVM region coordinates";
const COUNT_OVERFLOW: &str = "LLVM region count overflow";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Segment {
    pub line: u64,
    pub column: u64,
    pub count: u64,
    pub has_count: bool,
    pub is_region_entry: bool,
    pub is_gap_region: bool,
}

pub fn build_segments(input: &[Region]) -> Result<Vec<Segment>, SegmentError> {
    Ok(Builder::new(prepare_regions(input)?).build())
}

fn prepare_regions(input: &[Region]) -> Result<Vec<Region>, SegmentError> {
    let mut regions = input.to_vec();
    for region in &regions {
        let values = [
            region.start_line,
            region.start_column,
            region.end_line,
            region.end_column,
        ];
        if values
            .iter()
            .any(|value| !(MIN_COORDINATE..=MAX_COORDINATE).contains(value))
            || region.start() > region.end()
        {
            return Err(SegmentError::new(INVALID_REGION));
        }
    }
    regions.sort_by(|left, right| {
        left.start()
            .cmp(&right.start())
            .then_with(|| right.end().cmp(&left.end()))
            .then(left.kind.cmp(&right.kind))
    });
    let mut combined: Vec<Region> = Vec::with_capacity(regions.len());
    for region in regions {
        if let Some(active) = combined.last_mut()
            && active.start() == region.start()
            && active.end() == region.end()
        {
            if active.kind == region.kind {
                active.count = active
                    .count
                    .checked_add(region.count)
                    .ok_or_else(|| SegmentError::new(COUNT_OVERFLOW))?;
            }
            continue;
        }
        combined.push(region);
    }
    Ok(combined)
}

struct Builder {
    regions: Vec<Region>,
    active: Vec<usize>,
    output: Vec<Segment>,
}

impl Builder {
    fn new(regions: Vec<Region>) -> Self {
        Self {
            regions,
            active: Vec::new(),
            output: Vec::new(),
        }
    }

    fn emit(&mut self, region: usize, at: Position, entry: bool, skipped: bool) {
        let source = self.regions[region];
        let has_count = !skipped && source.kind != RegionKind::Skipped;
        if self.output.last().is_some_and(|last| {
            !entry
                && !skipped
                && last.has_count == has_count
                && last.count == source.count
                && !last.is_region_entry
        }) {
            return;
        }
        self.output.push(Segment {
            line: at.0,
            column: at.1,
            count: if has_count { source.count } else { ZERO_COUNT },
            has_count,
            is_region_entry: entry,
            is_gap_region: has_count && source.kind == RegionKind::Gap,
        });
    }

    fn finish_before(&mut self, at: Position) {
        let (mut open, completed): (Vec<_>, Vec<_>) = std::mem::take(&mut self.active)
            .into_iter()
            .partition(|region| self.regions[*region].end() > at);
        if completed.is_empty() {
            self.active = open;
            return;
        }
        let first = open.len();
        open.extend(completed);
        self.active = open;
        self.complete(Some(at), first);
    }

    fn complete(&mut self, at: Option<Position>, first: usize) {
        let regions = &self.regions;
        self.active[first..].sort_by_key(|region| regions[*region].end());
        for index in first + NEXT_INDEX..self.active.len() {
            let mut completed = self.active[index];
            let position = self.regions[self.active[index - NEXT_INDEX]].end();
            if at == Some(position) {
                break;
            }
            if position == self.regions[completed].end() {
                continue;
            }
            let completed_end = self.regions[completed].end();
            for candidate in &self.active[index + NEXT_INDEX..] {
                if completed_end == self.regions[*candidate].end() {
                    completed = *candidate;
                }
            }
            self.emit(completed, position, false, false);
        }
        let last = *self.active.last().expect("non-empty completed region set");
        let end = self.regions[last].end();
        if first > FIRST_INDEX && at != Some(end) {
            self.emit(self.active[first - NEXT_INDEX], end, false, false);
        } else if first == FIRST_INDEX && at != Some(end) {
            self.emit(last, end, false, true);
        }
        self.active.truncate(first);
    }

    fn build(mut self) -> Vec<Segment> {
        for index in FIRST_INDEX..self.regions.len() {
            let region = self.regions[index];
            self.finish_before(region.start());
            let next = index + NEXT_INDEX;
            let last = next == self.regions.len();
            let gap = region.kind == RegionKind::Gap;
            if region.start() == region.end() {
                let skipped = last || region.kind == RegionKind::Skipped;
                self.emit(
                    self.active.last().copied().unwrap_or(index),
                    region.start(),
                    !gap,
                    skipped,
                );
                if skipped && let Some(active) = self.active.last().copied() {
                    self.emit(active, region.start(), false, false);
                }
            } else {
                if last || region.start() != self.regions[next].start() {
                    self.emit(index, region.start(), !gap, false);
                }
                self.active.push(index);
            }
        }
        if !self.active.is_empty() {
            self.complete(None, FIRST_INDEX);
        }
        self.output
    }
}
