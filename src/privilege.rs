use privesc::PrivilegedCommand;
use std::env;
use std::io::Write;

pub fn ensure_root() {
    if is_root() {
        enable_env_privilege();
        return;
    }

    let exe = env::current_exe().expect("无法获取当前可执行文件路径");
    let args: Vec<String> = env::args().skip(1).collect();

    let output = PrivilegedCommand::new(&exe)
        .args(&args)
        .run()
        .expect("提权失败");

    if let Some(stdout) = &output.stdout {
        std::io::stdout().write_all(stdout).ok();
    }
    if let Some(stderr) = &output.stderr {
        std::io::stderr().write_all(stderr).ok();
    }
    std::process::exit(output.status.code().unwrap_or(1));
}

fn enable_env_privilege() {
    #[cfg(target_os = "windows")]
    {
        use std::ptr;
        const SE_SYSTEM_ENVIRONMENT_NAME: &str = "SeSystemEnvironmentPrivilege\0";
        const TOKEN_ADJUST_PRIVILEGES: u32 = 0x0020;
        const TOKEN_QUERY: u32 = 0x0008;
        const SE_PRIVILEGE_ENABLED: u32 = 0x0002;

        #[repr(C)]
        struct LUID {
            low: u32,
            high: i32,
        }
        #[repr(C)]
        struct LUID_AND_ATTRIBUTES {
            luid: LUID,
            attributes: u32,
        }
        #[repr(C)]
        struct TOKEN_PRIVILEGES {
            count: u32,
            privs: [LUID_AND_ATTRIBUTES; 1],
        }

        unsafe extern "system" {
            fn GetCurrentProcess() -> *const std::ffi::c_void;
            fn OpenProcessToken(
                handle: *const std::ffi::c_void,
                access: u32,
                token: *mut *mut std::ffi::c_void,
            ) -> i32;
            fn LookupPrivilegeValueW(system: *const u16, name: *const u16, luid: *mut LUID) -> i32;
            fn AdjustTokenPrivileges(
                token: *mut std::ffi::c_void,
                disable_all: i32,
                new_state: *mut TOKEN_PRIVILEGES,
                buf_len: u32,
                prev_state: *mut TOKEN_PRIVILEGES,
                ret_len: *mut u32,
            ) -> i32;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }

        let mut token: *mut std::ffi::c_void = ptr::null_mut();
        let ret = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &mut token,
            )
        };
        if ret == 0 {
            return;
        }

        let name_w: Vec<u16> = SE_SYSTEM_ENVIRONMENT_NAME.encode_utf16().collect();
        let mut luid = LUID { low: 0, high: 0 };
        let ret = unsafe { LookupPrivilegeValueW(ptr::null(), name_w.as_ptr(), &mut luid) };
        if ret == 0 {
            unsafe { CloseHandle(token) };
            return;
        }

        let mut tp = TOKEN_PRIVILEGES {
            count: 1,
            privs: [LUID_AND_ATTRIBUTES {
                luid,
                attributes: SE_PRIVILEGE_ENABLED,
            }],
        };
        unsafe {
            AdjustTokenPrivileges(token, 0, &mut tp, 0, ptr::null_mut(), ptr::null_mut());
            CloseHandle(token);
        }
    }
}

#[cfg(target_os = "linux")]
fn is_root() -> bool {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() == 0 }
}

#[cfg(target_os = "windows")]
fn is_root() -> bool {
    unsafe extern "system" {
        fn IsUserAnAdmin() -> i32;
    }
    unsafe { IsUserAnAdmin() != 0 }
}
