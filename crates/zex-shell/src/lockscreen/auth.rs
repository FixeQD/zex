//! PAM authentication

use std::ffi::CString;
use std::ptr;
use zeroize::Zeroizing;

const PAM_SUCCESS: libc::c_int = 0;
const PAM_CONV_ERR: libc::c_int = 19;
const PAM_PROMPT_ECHO_OFF: libc::c_int = 1;
const PAM_PROMPT_ECHO_ON: libc::c_int = 2;

#[repr(C)]
struct pam_message {
    msg_style: libc::c_int,
    msg: *const libc::c_char,
}

#[repr(C)]
struct pam_response {
    resp: *mut libc::c_char,
    resp_retcode: libc::c_int,
}

#[repr(C)]
struct pam_conv {
    conv: Option<
        unsafe extern "C" fn(
            libc::c_int,
            *const *const pam_message,
            *mut *mut pam_response,
            *mut libc::c_void,
        ) -> libc::c_int,
    >,
    appdata_ptr: *mut libc::c_void,
}

#[link(name = "pam")]
unsafe extern "C" {
    fn pam_start(
        service: *const libc::c_char,
        user: *const libc::c_char,
        conv: *const pam_conv,
        pamh: *mut *mut libc::c_void,
    ) -> libc::c_int;

    fn pam_authenticate(pamh: *mut libc::c_void, flags: libc::c_int) -> libc::c_int;

    fn pam_end(pamh: *mut libc::c_void, status: libc::c_int) -> libc::c_int;
}

unsafe extern "C" fn conv_callback(
    num_msg: libc::c_int,
    msg: *const *const pam_message,
    resp: *mut *mut pam_response,
    appdata_ptr: *mut libc::c_void,
) -> libc::c_int {
    unsafe {
        if num_msg <= 0 || msg.is_null() || resp.is_null() || appdata_ptr.is_null() {
            return PAM_CONV_ERR;
        }

        let n = num_msg as usize;
        let responses = libc::calloc(n, size_of::<pam_response>()) as *mut pam_response;
        if responses.is_null() {
            return PAM_CONV_ERR;
        }
        *resp = responses;

        let password = appdata_ptr as *const libc::c_char;
        let messages = *msg;

        for i in 0..n {
            let m = &*messages.add(i);
            let r = &mut *responses.add(i);
            match m.msg_style {
                PAM_PROMPT_ECHO_OFF | PAM_PROMPT_ECHO_ON => {
                    let dup = libc::strdup(password);
                    if dup.is_null() {
                        for j in 0..i {
                            libc::free((*responses.add(j)).resp as *mut _);
                        }
                        libc::free(responses as *mut _);
                        *resp = ptr::null_mut();
                        return PAM_CONV_ERR;
                    }
                    r.resp = dup;
                    r.resp_retcode = PAM_SUCCESS;
                }
                _ => {
                    r.resp = ptr::null_mut();
                    r.resp_retcode = PAM_SUCCESS;
                }
            }
        }

        PAM_SUCCESS
    }
}

/// Authenticate `user` with `password` against the PAM service `zex`
pub fn authenticate(user: &str, password: Zeroizing<String>) -> bool {
    let service = CString::new("zex").expect("CString::new failed");
    let user = CString::new(user).expect("CString::new failed");
    let password = CString::new(password.as_bytes()).expect("CString::new failed");

    let mut pamh: *mut libc::c_void = ptr::null_mut();

    unsafe {
        let conv = pam_conv {
            conv: Some(conv_callback),
            appdata_ptr: password.as_ptr() as *mut libc::c_void,
        };

        let ret = pam_start(service.as_ptr(), user.as_ptr(), &conv, &mut pamh);
        if ret != PAM_SUCCESS {
            return false;
        }

        let ret = pam_authenticate(pamh, 0);
        let ok = ret == PAM_SUCCESS;

        pam_end(pamh, ret);
        ok
    }
}

/// Login name of the process user: `$USER` when set, `getpwuid` otherwise
pub fn current_user() -> String {
    if let Ok(user) = std::env::var("USER")
        && !user.is_empty()
    {
        return user;
    }
    unsafe {
        let uid = libc::getuid();
        let pw = libc::getpwuid(uid);
        if !pw.is_null() && !(*pw).pw_name.is_null() {
            let name = std::ffi::CStr::from_ptr((*pw).pw_name);
            if let Ok(name) = name.to_str() {
                return name.to_string();
            }
        }
    }
    "unknown".to_string()
}
