use crate::constants::FUNCTION_REPORT_DIVIDER_SAMPLE as REPORT_DIVIDER;
use crate::test_constants::*;
use std::path::Path;

const REPORT_HEADER: &str = "Name Regions Miss Cover Lines Miss Cover Branches Miss Cover";
const REPORT_COUNTS: &str = "1 0 100% 1 0 100% 2 1 50%";

pub fn write(directory: &Path) {
    let metric = serde_json::json!({"count": 1, "covered": 1});
    let data = serde_json::json!({"data": [{
        "files": [{"filename": FIXTURE_SOURCE, "summary": {
            "lines": metric, "functions": metric, "regions": metric,
            "branches": {"count": 2, "covered": 1}
        }}],
        "functions": [{
            "name": FUNCTION, "count": 1, "filenames": [FIXTURE_SOURCE],
            "regions": [[1, 1, 1, 2, 1, 0, 0, 0]],
            "branches": [[1, 1, 1, 2, 1, 0, 0, 0, 0]]
        }]
    }]});
    std::fs::write(directory.join(JSON_FILE), data.to_string()).unwrap();
    std::fs::write(directory.join(LCOV_FILE), format!(
        "SF:{FIXTURE_SOURCE}\nFN:1,{FUNCTION}\nFNDA:1,{FUNCTION}\nFNF:1\nFNH:1\nDA:1,1\nLF:1\nLH:1\nBRDA:1,0,0,1\nBRDA:1,0,1,0\nBRF:2\nBRH:1\nend_of_record\n"
    )).unwrap();
    std::fs::write(directory.join(REPORT_FILE), format!(
        "File '{FIXTURE_SOURCE}':\n{REPORT_HEADER}\n{REPORT_DIVIDER}\n{FUNCTION} {REPORT_COUNTS}\n{REPORT_DIVIDER}\nTOTAL {REPORT_COUNTS}\n"
    )).unwrap();
}
