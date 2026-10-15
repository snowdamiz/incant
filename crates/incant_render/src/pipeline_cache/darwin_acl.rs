//! Darwin ACL admission on already-open descriptors. Never changes an ACL.
use std::{
    ffi::{c_int, c_long, c_void},
    io,
    os::fd::{AsRawFd, BorrowedFd},
    ptr::NonNull,
};

// Xcode sys/acl.h: opaque pointers, ACL_TYPE_EXTENDED=0x100, allow=1,
// deny=2, first=0, next=-1, ACL_MAX_ENTRIES=128. Darwin returns 0 for an
// entry and -1/EINVAL at exhaustion (unlike Linux's POSIX ACL iteration).
unsafe extern "C" {
    fn fpathconf(fd: c_int, name: c_int) -> c_long;
    fn filesec_init() -> *mut c_void;
    fn filesec_free(filesec: *mut c_void);
    // rustix::fs::Stat is libc::stat on Darwin. The SDK selects the INODE64
    // symbol on Intel; Apple Silicon has only the 64-bit inode ABI.
    #[cfg_attr(target_arch = "x86_64", link_name = "fstatx_np$INODE64")]
    fn fstatx_np(fd: c_int, stat: *mut rustix::fs::Stat, filesec: *mut c_void) -> c_int;
    fn filesec_query_property(filesec: *mut c_void, property: c_int, present: *mut c_int) -> c_int;
    fn filesec_get_property(filesec: *mut c_void, property: c_int, value: *mut c_void) -> c_int;
    fn acl_valid(acl: *mut c_void) -> c_int;
    fn acl_get_entry(acl: *mut c_void, index: c_int, entry: *mut *mut c_void) -> c_int;
    fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
    fn acl_free(acl: *mut c_void) -> c_int;
}
struct Acl(NonNull<c_void>);
impl Drop for Acl {
    fn drop(&mut self) {
        // SAFETY: owned ACL copy from filesec_get_property, released exactly once.
        unsafe {
            acl_free(self.0.as_ptr());
        }
    }
}
struct FileSecurity(NonNull<c_void>);
impl Drop for FileSecurity {
    fn drop(&mut self) {
        // SAFETY: owned filesec_init allocation; only releases working memory.
        unsafe { filesec_free(self.0.as_ptr()) };
    }
}
fn read_acl(fd: c_int) -> io::Result<Option<Acl>> {
    // SAFETY: fpathconf accepts a descriptor number and reports invalid FDs.
    // _PC_EXTENDED_SECURITY_NP=13 in the Darwin SDK. Unsupported or unknown
    // filesystem ACL capability disables persistence, even for an absent ACL.
    if unsafe { fpathconf(fd, 13) } != 1 {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "cannot establish Darwin extended-security support",
        ));
    }
    // SAFETY: allocates opaque working storage, freed by FileSecurity.
    let security =
        FileSecurity(NonNull::new(unsafe { filesec_init() }).ok_or_else(io::Error::last_os_error)?);
    let mut stat = std::mem::MaybeUninit::<rustix::fs::Stat>::uninit();
    // SAFETY: correctly sized/aligned native stat storage and live filesec.
    // No stat field is read; fstatx_np reports invalid descriptors normally.
    if unsafe { fstatx_np(fd, stat.as_mut_ptr(), security.0.as_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // acl_get_fd_np returns NULL/ENOENT for an absent FILESEC_ACL property as
    // well as genuine errors. Query the successful snapshot explicitly instead
    // of converting an undifferentiated ENOENT into permission to persist.
    let mut present = 0;
    // SAFETY: live snapshot and writable int output; FILESEC_ACL=5 in fcntl.h.
    if unsafe { filesec_query_property(security.0.as_ptr(), 5, &mut present) } != 0 {
        return Err(io::Error::last_os_error());
    }
    if present == 0 {
        return Ok(None);
    }
    let mut raw = std::ptr::null_mut::<c_void>();
    // SAFETY: the property is present and the output is an acl_t pointer slot.
    // The returned ACL is an independent allocation, not borrowed from filesec.
    if unsafe {
        filesec_get_property(
            security.0.as_ptr(),
            5,
            (&mut raw as *mut *mut c_void).cast(),
        )
    } != 0
    {
        return Err(io::Error::last_os_error());
    }
    // _FILESEC_REMOVE_ACL=((void *)1) in fcntl.h is not an ACL allocation.
    let pointer = NonNull::new(raw)
        .filter(|pointer| pointer.as_ptr().addr() != 1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing Darwin ACL copy"))?;
    Ok(Some(Acl(pointer)))
}
pub(super) fn deny_only(fd: BorrowedFd<'_>) -> io::Result<()> {
    let Some(acl) = read_acl(fd.as_raw_fd())? else {
        return Ok(());
    };
    // SAFETY: live owned ACL. Validate before treating EINVAL as exhaustion.
    if unsafe { acl_valid(acl.0.as_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    for index in 0..=128 {
        let mut entry = std::ptr::null_mut();
        // SAFETY: valid ACL, first/next iterator, writable output pointer.
        let status =
            unsafe { acl_get_entry(acl.0.as_ptr(), if index == 0 { 0 } else { -1 }, &mut entry) };
        if status == -1 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(rustix::io::Errno::INVAL.raw_os_error()) {
                return Ok(());
            }
            return Err(error);
        }
        if status != 0 || entry.is_null() || index == 128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid Darwin ACL iteration",
            ));
        }
        let mut tag = 0;
        // SAFETY: an entry returned by this still-live valid ACL.
        if unsafe { acl_get_tag_type(entry, &mut tag) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // Conservatively reject all grants, even owner-only or inherit-only
        // grants. Deny entries never expand mode-based access.
        if tag != 2 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "cache ACL grants access",
            ));
        }
    }
    unreachable!("bounded ACL loop returns before exhaustion")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};

    #[test]
    fn absent_acl_is_distinct_from_an_invalid_descriptor() {
        let base = std::env::var_os("INCANT_CACHE_TEST_ROOT")
            .or_else(|| std::env::var_os("HOME"))
            .expect("private test root");
        let root = tempfile::tempdir_in(base).unwrap();
        let path = root.path().join("no-acl");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        // Only this owned fixture is changed; make the absence precondition
        // explicit even if the test host's parent has inheritable deny entries.
        assert!(
            std::process::Command::new("/bin/chmod")
                .arg("-N")
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        assert!(read_acl(file.as_raw_fd()).unwrap().is_none());
        assert!(read_acl(-1).is_err());
    }
}
