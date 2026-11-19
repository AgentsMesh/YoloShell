use anyhow::Result;
#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
#[path = "resume_transaction_survivor_retire_test_hook_constants.rs"]
mod constants;
#[cfg(test)]
use constants::INVALID_BOUNDARY_ALIAS;

#[cfg(feature = "failed-rearm-test-hooks")]
pub const CRASH_BOUNDARY_ENVIRONMENT: &str = "YOLO_TEST_FAILED_REARM_CRASH_BOUNDARY";
#[cfg(test)]
const HOOK_ALREADY_INSTALLED: &str = "resume terminal rearm test hook is already installed";
const INVALID_BOUNDARY_DETAIL: &str = "invalid failed-resume rearm crash boundary";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    TombstoneRolled,
    PreviousLaunchRemoved,
    UncertainManifestPersisted,
    ReplacementTransactionPublished,
    ReplacementLaunchPublished,
}

impl Boundary {
    pub const COUNT: usize = 5;
    pub const ALL: [Self; Self::COUNT] = [
        Self::TombstoneRolled,
        Self::PreviousLaunchRemoved,
        Self::UncertainManifestPersisted,
        Self::ReplacementTransactionPublished,
        Self::ReplacementLaunchPublished,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TombstoneRolled => "tombstone_rolled",
            Self::PreviousLaunchRemoved => "previous_launch_removed",
            Self::UncertainManifestPersisted => "uncertain_manifest_persisted",
            Self::ReplacementTransactionPublished => "replacement_transaction_published",
            Self::ReplacementLaunchPublished => "replacement_launch_published",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        Self::ALL
            .into_iter()
            .find(|boundary| boundary.as_str() == value)
            .ok_or_else(|| anyhow::anyhow!("{INVALID_BOUNDARY_DETAIL}: {value}"))
    }
}

#[cfg(test)]
type Hook = Box<dyn FnMut(Boundary) -> Result<()>>;

#[cfg(test)]
thread_local! {
    static HOOK: RefCell<Option<Hook>> = const { RefCell::new(None) };
}

#[cfg(test)]
pub(crate) struct Guard(std::marker::PhantomData<std::rc::Rc<()>>);

#[cfg(test)]
pub(crate) fn install(hook: impl FnMut(Boundary) -> Result<()> + 'static) -> Guard {
    HOOK.with(|slot| {
        let previous = slot.borrow_mut().replace(Box::new(hook));
        assert!(previous.is_none(), "{HOOK_ALREADY_INSTALLED}");
    });
    Guard(std::marker::PhantomData)
}

#[cfg(feature = "failed-rearm-test-hooks")]
pub fn configured_boundary() -> Result<Option<Boundary>> {
    let Some(value) = std::env::var_os(CRASH_BOUNDARY_ENVIRONMENT) else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_| anyhow::anyhow!(INVALID_BOUNDARY_DETAIL))?;
    Boundary::parse(&value).map(Some)
}

pub(super) fn run(boundary: Boundary) -> Result<()> {
    #[cfg(test)]
    if let Some(result) = HOOK.with(|slot| slot.borrow_mut().as_mut().map(|hook| hook(boundary))) {
        return result;
    }
    #[cfg(feature = "failed-rearm-test-hooks")]
    if configured_boundary()? == Some(boundary) {
        yolo_process_test_support::send_signal(std::process::id(), libc::SIGKILL)?;
        loop {
            std::thread::park();
        }
    }
    Ok(())
}

#[cfg(test)]
impl Drop for Guard {
    fn drop(&mut self) {
        HOOK.with(|slot| *slot.borrow_mut() = None);
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/resume/recovery/resume_transaction_survivor_retire_test_hook_tests.rs"]
mod tests;
