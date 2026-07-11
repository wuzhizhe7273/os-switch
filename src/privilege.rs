use privesc::PrivilegedCommand;
use std::env;
use std::io::Write;

pub fn ensure_root() {
    if is_root() {
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
