use super::*;
use portable_pty::{PtySize, native_pty_system};

const TEST_CAPABILITY: &str = "master-custody-probe-capability";
const TEST_ROWS: u16 = 24;
const TEST_COLUMNS: u16 = 80;
const TEST_PIXEL_SIZE: u16 = 0;

#[test]
fn probe_tracks_the_exact_retained_passive_master_descriptor() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: TEST_ROWS,
            cols: TEST_COLUMNS,
            pixel_width: TEST_PIXEL_SIZE,
            pixel_height: TEST_PIXEL_SIZE,
        })
        .unwrap();
    let proof = super::super::PtySessionProof::capture_bound(
        pair.master.as_ref(),
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let passive = MacosPassivePtyMaster::duplicate_exact(&proof).unwrap();
    let probe = PtyMasterFdCustodyProbe::capture(&passive).unwrap();
    drop(pair);
    drop(proof);

    assert!(probe.process_holds_pty_object(std::process::id()).unwrap());
    drop(passive);
    assert!(!probe.process_holds_pty_object(std::process::id()).unwrap());
}
