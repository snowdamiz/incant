//! Handle-based owner/protected-DACL admission. No existing ACL is repaired.
use std::{ffi::c_void, fs::File, io, os::windows::io::AsRawHandle, ptr};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, LocalFree},
    Security::{
        self, ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, Authorization::*,
        DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
        SE_DACL_PROTECTED, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
    },
    System::{
        SystemServices::ACCESS_ALLOWED_ACE_TYPE,
        Threading::{GetCurrentProcess, OpenProcessToken},
    },
};

fn refused(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::PermissionDenied, message)
}
struct Local(PSECURITY_DESCRIPTOR);
impl Drop for Local {
    fn drop(&mut self) {
        // SAFETY: allocation returned by GetSecurityInfo/SDDL conversion.
        unsafe {
            LocalFree(self.0);
        }
    }
}
struct Token(HANDLE);
impl Drop for Token {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
pub(super) fn user() -> io::Result<Vec<u8>> {
    let mut token = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = Token(token);
    let mut required = 0;
    unsafe {
        Security::GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required < size_of::<TOKEN_USER>() as u32 || required > 64 * 1024 {
        return Err(refused("invalid current-user token size"));
    }
    // A word-aligned allocation is required for TOKEN_USER, not Vec<u8> casts.
    let mut words = vec![0usize; (required as usize).div_ceil(size_of::<usize>())];
    if unsafe {
        Security::GetTokenInformation(
            token.0,
            TokenUser,
            words.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let sid = unsafe { (*(words.as_ptr().cast::<TOKEN_USER>())).User.Sid };
    copy_sid(sid)
}
fn copy_sid(sid: PSID) -> io::Result<Vec<u8>> {
    if sid.is_null() || unsafe { Security::IsValidSid(sid) } == 0 {
        return Err(refused("invalid security SID"));
    }
    let size = unsafe { Security::GetLengthSid(sid) };
    let mut bytes = vec![0; size as usize];
    if unsafe { Security::CopySid(size, bytes.as_mut_ptr().cast(), sid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(bytes)
}
pub(super) fn validate(file: &File, user: &[u8]) -> io::Result<()> {
    let mut owner = ptr::null_mut();
    let mut acl: *mut ACL = ptr::null_mut();
    let mut descriptor = ptr::null_mut();
    let status = unsafe {
        GetSecurityInfo(
            file.as_raw_handle(),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            ptr::null_mut(),
            &mut acl,
            ptr::null_mut(),
            &mut descriptor,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let descriptor = Local(descriptor);
    if owner.is_null() || unsafe { Security::EqualSid(owner, user.as_ptr().cast_mut().cast()) } == 0
    {
        return Err(refused("cache object is not owned by the current user"));
    }
    let mut control = 0;
    let mut revision = 0;
    if unsafe { Security::GetSecurityDescriptorControl(descriptor.0, &mut control, &mut revision) }
        == 0
    {
        return Err(io::Error::last_os_error());
    }
    if control & SE_DACL_PROTECTED == 0 || acl.is_null() {
        return Err(refused(
            "cache DACL must be explicit, protected and nonnull",
        ));
    }
    let mut info = ACL_SIZE_INFORMATION::default();
    if unsafe {
        Security::GetAclInformation(
            acl,
            (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            Security::AclSizeInformation,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    if info.AceCount == 0 || info.AceCount > 32 {
        return Err(refused("cache DACL ACE count"));
    }
    let mut owner_allowed = false;
    for index in 0..info.AceCount {
        let mut pointer: *mut c_void = ptr::null_mut();
        if unsafe { Security::GetAce(acl, index, &mut pointer) } == 0 {
            return Err(io::Error::last_os_error());
        }
        let header = unsafe { &*pointer.cast::<ACE_HEADER>() };
        // Refuse inherited, object, conditional, callback, deny and unknown ACEs
        // rather than guessing their Windows authorization semantics.
        if u32::from(header.AceType) != ACCESS_ALLOWED_ACE_TYPE
            || header.AceFlags != 0
            || usize::from(header.AceSize) < 16
        // mask/header + minimum SID header
        {
            return Err(refused("cache DACL has unsupported or inherited ACEs"));
        }
        let ace = unsafe { &*pointer.cast::<ACCESS_ALLOWED_ACE>() };
        let sid = ptr::addr_of!(ace.SidStart).cast_mut().cast();
        // Check the SID's fixed header and declared subauthorities against the
        // ACE before calling native SID validation/copy routines.
        let sid_length = 8 + 4 * usize::from(unsafe { *sid.cast::<u8>().add(1) });
        if 8 + sid_length > usize::from(header.AceSize) {
            return Err(refused("cache DACL SID exceeds ACE"));
        }
        let bytes = copy_sid(sid)?;
        if 8 + bytes.len() > usize::from(header.AceSize) {
            return Err(refused("cache DACL SID exceeds ACE"));
        }
        if bytes == user {
            owner_allowed |= ace.Mask != 0;
        } else {
            // SYSTEM/Administrators are the only additional trusted principals.
            let system = unsafe { Security::IsWellKnownSid(sid, Security::WinLocalSystemSid) } != 0;
            let admins =
                unsafe { Security::IsWellKnownSid(sid, Security::WinBuiltinAdministratorsSid) }
                    != 0;
            if !system && !admins {
                return Err(refused("cache DACL grants another principal access"));
            }
        }
    }
    if !owner_allowed {
        return Err(refused("cache DACL has no current-user grant"));
    }
    Ok(())
}

pub(super) fn create<T>(
    user: &[u8],
    create: impl FnOnce(*const SECURITY_ATTRIBUTES) -> io::Result<T>,
) -> io::Result<T> {
    let mut string = ptr::null_mut();
    if unsafe { ConvertSidToStringSidW(user.as_ptr().cast_mut().cast(), &mut string) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let allocation = Local(string.cast());
    let mut length = 0;
    while unsafe { *string.add(length) } != 0 {
        length += 1;
        if length > 184 {
            return Err(refused("SID string exceeds bound"));
        }
    }
    let sid = String::from_utf16(unsafe { std::slice::from_raw_parts(string, length) })
        .map_err(|_| refused("SID string is not UTF-16"))?;
    drop(allocation);
    let sddl: Vec<u16> = format!("O:{sid}D:P(A;;FA;;;{sid})(A;;FA;;;SY)(A;;FA;;;BA)")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let descriptor = Local(descriptor);
    let attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: 0,
    };
    create(&attributes)
}

#[cfg(test)]
pub(super) fn replace_test_dacl(
    path: &std::path::Path,
    sddl: &str,
    protected: bool,
) -> io::Result<()> {
    let sddl: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let mut descriptor = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let descriptor = Local(descriptor);
    let path = super::wide(path)?;
    let flags = if protected {
        Security::PROTECTED_DACL_SECURITY_INFORMATION
    } else {
        Security::UNPROTECTED_DACL_SECURITY_INFORMATION
    };
    if unsafe {
        Security::SetFileSecurityW(
            path.as_ptr(),
            DACL_SECURITY_INFORMATION | flags,
            descriptor.0,
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_owner_check_refuses_another_valid_sid_without_changing_any_acl() {
        let root = tempfile::tempdir().unwrap();
        let directory = super::super::Directory::open(&root.path().join("library"), true).unwrap();
        let file = directory.chain.last().unwrap();
        validate(file, &user().unwrap()).unwrap();
        let mut size = Security::SECURITY_MAX_SID_SIZE;
        let mut sid = vec![0_u8; size as usize];
        assert_ne!(
            unsafe {
                Security::CreateWellKnownSid(
                    Security::WinLocalSystemSid,
                    ptr::null_mut(),
                    sid.as_mut_ptr().cast(),
                    &mut size,
                )
            },
            0
        );
        sid.truncate(size as usize);
        assert_eq!(
            validate(file, &sid).unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
        validate(file, &user().unwrap()).unwrap();
    }
}
