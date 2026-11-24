use anyhow::{Context, Result};
use std::ffi::{CStr, CString, c_char, c_void};

type Object = *mut c_void;
type Selector = *mut c_void;
type SendObject = unsafe extern "C" fn(Object, Selector) -> Object;
type SendDirectories = unsafe extern "C" fn(Object, Selector, usize, usize) -> Object;
const APPLICATION_SUPPORT_DIRECTORY: usize = 14;
const USER_DOMAIN_MASK: usize = 1;
const DEFAULT_MANAGER: &CStr = c"defaultManager";
const URLS_FOR_DIRECTORY: &CStr = c"URLsForDirectory:inDomains:";
const FIRST_OBJECT: &CStr = c"firstObject";
const FILE_SYSTEM_REPRESENTATION: &CStr = c"fileSystemRepresentation";

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    #[link_name = "OBJC_CLASS_$_NSFileManager"]
    static FILE_MANAGER_CLASS: c_void;
}
#[link(name = "objc")]
unsafe extern "C" {
    fn sel_registerName(name: *const c_char) -> Selector;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> Object;
    fn objc_autoreleasePoolPop(pool: Object);
}

struct Pool(Object);
impl Drop for Pool {
    fn drop(&mut self) {
        // The pool is created and destroyed synchronously on this same thread.
        unsafe { objc_autoreleasePoolPop(self.0) };
    }
}

fn nonnull(value: Object) -> Result<Object> {
    (!value.is_null())
        .then_some(value)
        .context(crate::constants::FOUNDATION_UNAVAILABLE)
}

unsafe fn object(receiver: Object, selector: &CStr) -> Result<Object> {
    // Cast to the exact non-variadic Objective-C ABI, including on Apple arm64.
    let send: SendObject = unsafe { std::mem::transmute(objc_msgSend as unsafe extern "C" fn()) };
    nonnull(unsafe { send(receiver, sel_registerName(selector.as_ptr())) })
}

pub(super) fn application_support() -> Result<CString> {
    // Foundation objects and borrowed representation remain inside the autorelease pool.
    unsafe {
        let _pool = Pool(nonnull(objc_autoreleasePoolPush())?);
        let manager = object(
            std::ptr::addr_of!(FILE_MANAGER_CLASS).cast_mut(),
            DEFAULT_MANAGER,
        )?;
        let send: SendDirectories = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
        let urls = nonnull(send(
            manager,
            sel_registerName(URLS_FOR_DIRECTORY.as_ptr()),
            APPLICATION_SUPPORT_DIRECTORY,
            USER_DOMAIN_MASK,
        ))?;
        let url = object(urls, FIRST_OBJECT)?;
        let representation = object(url, FILE_SYSTEM_REPRESENTATION)?.cast::<c_char>();
        Ok(CStr::from_ptr(representation).to_owned())
    }
}
