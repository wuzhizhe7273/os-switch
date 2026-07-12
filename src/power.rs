use std::process::Command;

use crate::boot::BootManager;
use crate::error::BootError;

const LINUX_REBOOT_CMD_RESTART: i32 = 0x01234567;

unsafe extern "C" {
    fn sync();
    fn reboot(cmd: i32) -> i32;
}

/// 休眠后关机。成功永不返回，失败返回错误
pub fn hibernate() -> Result<(), BootError> {
    unsafe {
        sync();
    }

    Command::new("systemctl")
        .args(["hibernate"])
        .status()
        .map_err(|e| BootError::HibernateFailed(format!("无法执行 systemctl: {e}")))?;

    std::process::exit(0);
}

/// 直接重启（不保存会话）
pub fn reboot_system() -> ! {
    unsafe {
        sync();
        reboot(LINUX_REBOOT_CMD_RESTART);
    }
    std::process::exit(1);
}

/// 尝试休眠，失败时清除 BootNext 并返回错误
pub fn try_hibernate(mgr: &dyn BootManager) -> Result<(), BootError> {
    match hibernate() {
        Ok(()) => unreachable!(),
        Err(e) => {
            mgr.clear_next_boot()?;
            Err(e)
        }
    }
}
