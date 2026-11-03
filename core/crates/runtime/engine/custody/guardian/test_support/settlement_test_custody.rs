use super::test_support::Fixture;
use crate::process::{ActiveMacosSecondaryCustodyFixture, MacosSecondaryCustody};
use crate::pty_custody::CustodyRuntimeIdentity;
use portable_pty::PtySize;

const PTY_PIXEL_SIZE: u16 = 0;
const TEST_CAPABILITY: &str = "guardian-settlement-test";

pub(super) fn custody(fixture: &Fixture) -> MacosSecondaryCustody {
    let fixture_custody = ActiveMacosSecondaryCustodyFixture::new(
        PtySize {
            rows: fixture.runtime.rows,
            cols: fixture.runtime.cols,
            pixel_width: PTY_PIXEL_SIZE,
            pixel_height: PTY_PIXEL_SIZE,
        },
        TEST_CAPABILITY,
    );
    let runtime = CustodyRuntimeIdentity::from_record(&fixture.runtime);
    fixture_custody.capture(&runtime)
}
