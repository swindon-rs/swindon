use std::io;
use crate::config::Config;


#[cfg(unix)]
pub fn drop(cfg: &Config) -> Result<(), io::Error> {
    use libc::{getpwnam, getgrnam, setuid, setgid};
    use std::ffi::CString;
    use std::ptr;

    if let Some(ref group) = cfg.set_group {
        unsafe {
            let grstring = CString::new(group.as_bytes()).unwrap();
            let gentry = getgrnam(grstring.as_ptr());
            if gentry == ptr::null_mut() {
                return Err(io::Error::last_os_error());
            }
            info!("Group {:?} has gid of {}", group, (*gentry).gr_gid);
            if setgid((*gentry).gr_gid) == -1 {
                return Err(io::Error::last_os_error());
            }
        }
    }

    if let Some(ref user) = cfg.set_user {
        unsafe {
            let ustring = CString::new(user.as_bytes()).unwrap();
            let uentry = getpwnam(ustring.as_ptr());
            if uentry == ptr::null_mut() {
                return Err(io::Error::last_os_error());
            }
            if cfg.set_group.is_none() {
                info!("User {:?} has uid of {} and primary group {}",
                    user, (*uentry).pw_uid, (*uentry).pw_gid);
                if setgid((*uentry).pw_gid) == -1 {
                    return Err(io::Error::last_os_error());
                }
            } else {
                info!("User {:?} has uid of {}", user, (*uentry).pw_uid);
            }
            if setuid((*uentry).pw_uid) == -1 {
                return Err(io::Error::last_os_error());
            }
        }
    }
    Ok(())
}
#[cfg(not(unix))]
pub fn drop(_: &Config) -> Result<(), io::Error> {
    Ok(())
}
