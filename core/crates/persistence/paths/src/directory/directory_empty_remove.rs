use super::{PrivateDirectory, entry_c_string};
use anyhow::{Context, Result, ensure};
use std::mem::MaybeUninit;

const DESCENDANT_INVALID: &str = "private empty descendant changed";
const REMOVE_CONTEXT: &str = "remove exact empty private descendant";
const SYSTEM_CALL_FAILURE: libc::c_int = -1;

impl PrivateDirectory {
    pub fn remove_empty_descendant(&self, expected: &Self) -> Result<bool> {
        ensure!(
            expected.path().parent() == Some(self.path()),
            DESCENDANT_INVALID
        );
        let name = entry_c_string(expected.path().file_name().context(DESCENDANT_INVALID)?)?;
        self.verify_path().context(DESCENDANT_INVALID)?;
        if !self.matches_empty_descendant(expected)? {
            return Ok(false);
        }
        run_before_remove_hook();
        self.verify_path().context(DESCENDANT_INVALID)?;
        if !self.matches_empty_descendant(expected)? {
            return Ok(false);
        }
        let result =
            unsafe { libc::unlinkat(self.descriptor(), name.as_ptr(), libc::AT_REMOVEDIR) };
        ensure!(result != SYSTEM_CALL_FAILURE, REMOVE_CONTEXT);
        self.sync().context(REMOVE_CONTEXT)?;
        ensure!(entry_missing(self.descriptor(), &name)?, DESCENDANT_INVALID);
        self.verify_path().context(DESCENDANT_INVALID)?;
        Ok(true)
    }

    fn matches_empty_descendant(&self, expected: &Self) -> Result<bool> {
        let current = self.open_descendant(expected.path())?;
        if !current.same_identity(expected) {
            return Ok(false);
        }
        let revision = current.revision()?;
        let mut stream =
            crate::paths::directory_stream::DirectoryStream::open(current.descriptor())?;
        let empty = stream.next_name()?.is_none();
        drop(stream);
        current.verify_path()?;
        ensure!(current.revision()? == revision, DESCENDANT_INVALID);
        Ok(empty)
    }
}

fn entry_missing(parent: std::os::fd::RawFd, name: &std::ffi::CStr) -> Result<bool> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != SYSTEM_CALL_FAILURE {
        return Ok(false);
    }
    let error = std::io::Error::last_os_error();
    ensure!(
        error.kind() == std::io::ErrorKind::NotFound,
        DESCENDANT_INVALID
    );
    Ok(true)
}

#[cfg(any(test, feature = "persistence-test-support"))]
thread_local! {
    static BEFORE_REMOVE: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "persistence-test-support"))]
pub(crate) fn set_before_remove_hook(hook: impl FnOnce() + 'static) {
    BEFORE_REMOVE.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

fn run_before_remove_hook() {
    #[cfg(any(test, feature = "persistence-test-support"))]
    BEFORE_REMOVE.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}
