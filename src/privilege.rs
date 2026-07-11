use std::env;
use std::ffi::CString;

unsafe extern "C" {
    fn geteuid() -> u32;
    fn execvp(file: *const i8, argv: *const *const i8) -> i32;
}

pub fn ensure_root() {
    if unsafe { geteuid() == 0 } {
        return;
    }

    let args: Vec<CString> = env::args()
        .map(|a| CString::new(a).expect("参数包含 null 字节"))
        .collect();

    let sudo = CString::new("sudo").expect("sudo");
    let mut argv: Vec<*const i8> = vec![sudo.as_ptr()];
    for a in &args {
        argv.push(a.as_ptr());
    }
    argv.push(std::ptr::null());

    unsafe {
        execvp(sudo.as_ptr(), argv.as_ptr());
    }
    eprintln!("提权失败: 无法执行 sudo");
    std::process::exit(1);
}
