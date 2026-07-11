#[cfg(target_os = "linux")]
mod imp {
    use crate::error::BootError;
    use std::process::Command;

    const LINUX_REBOOT_CMD_RESTART: i32 = 0x01234567;

    unsafe extern "C" {
        fn sync();
        fn reboot(cmd: i32) -> i32;
    }

    pub fn hibernate_reboot() -> Result<(), BootError> {
        unsafe {
            sync();
        }

        let status = Command::new("systemctl")
            .args(["hibernate"])
            .status()
            .map_err(|e| BootError::HibernateFailed(format!("无法执行 systemctl: {e}")))?;

        if status.success() {
            unreachable!()
        } else {
            Err(BootError::HibernateFailed(
                "systemctl hibernate 返回非零".into(),
            ))
        }
    }

    pub fn reboot_now() -> ! {
        unsafe {
            sync();
            reboot(LINUX_REBOOT_CMD_RESTART);
        }
        std::process::exit(1);
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use crate::error::BootError;

    #[link(name = "powrprof")]
    unsafe extern "system" {
        fn SetSuspendState(bHibernate: i32, bForce: i32, bDisableWakeEvent: i32) -> i32;
    }

    pub fn hibernate_reboot() -> Result<(), BootError> {
        let ret = unsafe { SetSuspendState(1, 1, 0) };
        if ret == 0 {
            Err(BootError::HibernateFailed("SetSuspendState 失败".into()))
        } else {
            unreachable!()
        }
    }

    pub fn reboot_now() -> ! {
        let _ = std::process::Command::new("shutdown")
            .args(["/r", "/t", "0"])
            .status();
        std::process::exit(1);
    }
}

pub use imp::*;
