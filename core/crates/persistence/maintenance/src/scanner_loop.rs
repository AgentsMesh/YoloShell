use super::ScannerSet;
use super::control::StopControl;
use super::scanner_containment::{catch, discard, error_detail, trace};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use yolo_maintenance_contract::{MaintenanceScanner, ScanChunk};

const BUSY_RETRY_INTERVAL: Duration = Duration::from_millis(5);
const ERROR_RETRY_INITIAL: Duration = Duration::from_millis(25);
const ERROR_RETRY_MAX: Duration = Duration::from_secs(5);
#[cfg(not(test))]
const PASS_INTERVAL: Duration = Duration::from_secs(30);
#[cfg(test)]
const PASS_INTERVAL: Duration = Duration::from_millis(10);
const DROP_PANIC_MESSAGE: &str = "quarantined maintenance scanner panicked while dropping";
const PANIC_MESSAGE: &str = "maintenance scanner panicked; restarting its pass";
const RESET_PANIC_MESSAGE: &str = "maintenance scanner reset panicked; quarantining scanner";
const SCAN_FAILED_MESSAGE: &str = "maintenance scan failed";

struct Schedule {
    next: Instant,
    error_delay: Duration,
}

enum ScanAction {
    Continue,
    Immediate,
    Quarantine,
}

impl Schedule {
    fn ready() -> Self {
        Self {
            next: Instant::now(),
            error_delay: ERROR_RETRY_INITIAL,
        }
    }

    fn after_success(&mut self, delay: Duration) {
        self.next = Instant::now() + delay;
        self.error_delay = ERROR_RETRY_INITIAL;
    }

    fn after_failure(&mut self) {
        self.next = Instant::now() + self.error_delay;
        self.error_delay = self
            .error_delay
            .saturating_add(self.error_delay)
            .min(ERROR_RETRY_MAX);
    }
}

pub(super) fn run_as_owner(
    scanners: &mut ScannerSet,
    control: &StopControl,
    reset_panicked: &AtomicBool,
) {
    let mut schedules: Vec<_> = scanners.iter().map(|_| Schedule::ready()).collect();
    while !control.requested() && !scanners.is_empty() {
        let now = Instant::now();
        let mut immediate = false;
        let mut index = 0;
        while index < scanners.len() {
            if schedules[index].next > now || control.requested() {
                index += 1;
                continue;
            }
            match scan_one(&mut *scanners[index], &mut schedules[index], reset_panicked) {
                ScanAction::Continue => index += 1,
                ScanAction::Immediate => {
                    immediate = true;
                    index += 1;
                }
                ScanAction::Quarantine => quarantine(scanners, &mut schedules, index),
            }
        }
        if scanners.is_empty() {
            return;
        }
        if immediate {
            std::thread::yield_now();
            continue;
        }
        let delay = schedules
            .iter()
            .map(|schedule| schedule.next)
            .min()
            .map(|next| next.saturating_duration_since(Instant::now()))
            .unwrap_or(PASS_INTERVAL);
        control.wait(delay);
    }
    while let Some(scanner) = scanners.pop() {
        drop_scanner(scanner);
    }
}

fn scan_one(
    scanner: &mut dyn MaintenanceScanner,
    schedule: &mut Schedule,
    reset_panicked: &AtomicBool,
) -> ScanAction {
    let label = scanner.label();
    match catch(|| scanner.scan_chunk()) {
        Ok(Ok(ScanChunk::Busy)) => {
            schedule.after_success(BUSY_RETRY_INTERVAL);
            ScanAction::Continue
        }
        Ok(Ok(ScanChunk::Progress)) => {
            schedule.after_success(Duration::ZERO);
            ScanAction::Immediate
        }
        Ok(Ok(ScanChunk::PassComplete)) => {
            schedule.after_success(PASS_INTERVAL);
            ScanAction::Continue
        }
        Ok(Err(error)) => {
            let detail = error_detail(error);
            trace(|| tracing::warn!(%detail, scanner = label, SCAN_FAILED_MESSAGE));
            reset(scanner, schedule, label, reset_panicked)
        }
        Err(()) => {
            trace(|| tracing::error!(scanner = label, PANIC_MESSAGE));
            reset(scanner, schedule, label, reset_panicked)
        }
    }
}

fn reset(
    scanner: &mut dyn MaintenanceScanner,
    schedule: &mut Schedule,
    label: &'static str,
    reset_panicked: &AtomicBool,
) -> ScanAction {
    if catch(|| scanner.reset()).is_err() {
        reset_panicked.store(true, Ordering::Release);
        trace(|| tracing::error!(scanner = label, RESET_PANIC_MESSAGE));
        return ScanAction::Quarantine;
    }
    schedule.after_failure();
    ScanAction::Continue
}

fn quarantine(scanners: &mut ScannerSet, schedules: &mut Vec<Schedule>, index: usize) {
    let scanner = scanners.swap_remove(index);
    schedules.swap_remove(index);
    drop_scanner(scanner);
}

fn drop_scanner(scanner: Box<dyn MaintenanceScanner>) {
    if !discard(scanner) {
        trace(|| tracing::error!(DROP_PANIC_MESSAGE));
    }
}
