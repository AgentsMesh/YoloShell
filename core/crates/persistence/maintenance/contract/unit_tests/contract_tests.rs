use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk, ScannerSet};

const TEST_SCANNER_LABEL: &str = "contract-test-scanner";
const TEST_SCAN_SEQUENCE: [ScanChunk; 3] = [
    ScanChunk::Busy,
    ScanChunk::Progress,
    ScanChunk::PassComplete,
];

struct TestScanner {
    reset: Arc<AtomicBool>,
    chunks: VecDeque<ScanChunk>,
}

impl MaintenanceScanner for TestScanner {
    fn label(&self) -> &'static str {
        TEST_SCANNER_LABEL
    }

    fn scan_chunk(&mut self) -> Result<ScanChunk> {
        Ok(self.chunks.pop_front().unwrap_or(ScanChunk::PassComplete))
    }

    fn reset(&mut self) {
        self.reset.store(true, Ordering::SeqCst);
    }
}

#[test]
fn scanner_contract_is_object_safe_and_preserves_progress_states() {
    let reset = Arc::new(AtomicBool::new(false));
    let mut scanners: ScannerSet = vec![Box::new(TestScanner {
        reset: reset.clone(),
        chunks: TEST_SCAN_SEQUENCE.into(),
    })];
    let scanner = scanners.first_mut().unwrap();
    assert_eq!(scanner.label(), TEST_SCANNER_LABEL);
    for expected in TEST_SCAN_SEQUENCE {
        assert_eq!(scanner.scan_chunk().unwrap(), expected);
    }
    scanner.reset();
    assert!(reset.load(Ordering::SeqCst));
}
