use crate::constants::{
    BRANCHES_LABEL, FUNCTIONS_LABEL, LINES_LABEL, PERCENT_SCALE, REGIONS_LABEL, REQUIRED_PERCENT,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Metric {
    pub found: u64,
    pub hit: u64,
}

impl Metric {
    pub fn new(found: u64, hit: u64, label: &str) -> Result<Self, String> {
        if hit > found {
            return Err(format!(
                "{label} hit count {hit} exceeds found count {found}"
            ));
        }
        Ok(Self { found, hit })
    }

    pub fn checked_add(self, other: Self, label: &str) -> Result<Self, String> {
        let found = self
            .found
            .checked_add(other.found)
            .ok_or_else(|| format!("{label} found count overflow"))?;
        let hit = self
            .hit
            .checked_add(other.hit)
            .ok_or_else(|| format!("{label} hit count overflow"))?;
        Self::new(found, hit, label)
    }

    pub fn percentage(self) -> f64 {
        self.hit as f64 / self.found as f64 * PERCENT_SCALE as f64
    }

    pub fn passes(self) -> bool {
        u128::from(self.hit) * u128::from(PERCENT_SCALE)
            > u128::from(self.found) * u128::from(REQUIRED_PERCENT)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Totals {
    pub lines: Metric,
    pub functions: Metric,
    pub regions: Metric,
    pub branches: Metric,
}

impl Totals {
    pub fn new(
        lines: Metric,
        functions: Metric,
        regions: Metric,
        branches: Metric,
    ) -> Result<Self, String> {
        let totals = Self {
            lines,
            functions,
            regions,
            branches,
        };
        for (label, metric) in totals.metrics() {
            if metric.found == 0 {
                return Err(format!("{label} coverage has a zero denominator"));
            }
        }
        Ok(totals)
    }

    pub fn passes(self) -> bool {
        self.metrics()
            .into_iter()
            .all(|(_, metric)| metric.passes())
    }

    pub fn metrics(self) -> [(&'static str, Metric); 4] {
        [
            (LINES_LABEL, self.lines),
            (FUNCTIONS_LABEL, self.functions),
            (REGIONS_LABEL, self.regions),
            (BRANCHES_LABEL, self.branches),
        ]
    }
}
