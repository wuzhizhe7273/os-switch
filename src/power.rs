use std::process::Command;

use crate::error::BootError;

const LINUX_REBOOT_CMD_RESTART: i32 = 0x01234567;

unsafe extern "C" {
    fn sync();
    fn reboot(cmd: i32) -> i32;
}

/// 休眠后关机，成功则永不返回
pub fn hibernate_reboot() -> Result<(), BootError> {
    unsafe {
        sync();
    }

    // systemctl hibernate 发送请求给 logind 后立即返回，logind 异步执行休眠
    Command::new("systemctl")
        .args(["hibernate"])
        .status()
        .map_err(|e| BootError::HibernateFailed(format!("无法执行 systemctl: {e}")))?;

    std::process::exit(0);
}

/// 直接重启（不保存会话）
pub fn reboot_now() -> ! {
    unsafe {
        sync();
        reboot(LINUX_REBOOT_CMD_RESTART);
    }
    std::process::exit(1);
}
