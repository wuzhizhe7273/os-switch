use crate::boot::{BootEntry, BootManager};
use crate::error::BootError;

/// 命令执行结果（数据，不负责展示）
pub enum Output {
    EntryList(Vec<BootEntry>),
    Status {
        manager_name: String,
        entry_count: usize,
        boot_next: Option<(String, String)>, // (id, description)
    },
    SwitchResult {
        target: String,
        boot_next_id: String,
    },
    CancelResult,
}

pub trait Cmd {
    fn run(&self, mgr: &dyn BootManager) -> Result<Output, BootError>;
}

pub mod cancel;
pub mod list;
pub mod status;
pub mod switch_cmd;
