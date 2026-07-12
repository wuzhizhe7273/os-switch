use clap::{Parser, Subcommand};

use crate::cmd::{self, Cmd};

#[derive(Parser)]
#[command(name = "os-switch", about = "双系统快速切换工具")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// 列出所有可用引导项
    List,
    /// 显示当前系统状态
    Status,
    /// 设置下次引导目标（不触发休眠/重启）
    Set {
        /// 目标引导项名称（description 或配置别名）
        name: String,
    },
    /// 切换到目标系统
    Switch {
        /// 目标引导项名称（description 或配置别名）
        name: String,
        /// 直接重启而非休眠
        #[arg(long)]
        reboot: bool,
    },
    /// 清除已设置的一次性引导
    Cancel,
}

impl Command {
    /// 是否需要 root 权限
    pub fn needs_root(&self) -> bool {
        matches!(
            self,
            Command::Switch { .. } | Command::Cancel | Command::Set { .. }
        )
    }

    /// 是否需要触发休眠/重启
    pub fn needs_power(&self) -> bool {
        matches!(self, Command::Switch { .. })
    }

    /// 分发到具体命令，返回 (Cmd, reboot 标志, 目标名)
    pub fn into_cmd(self) -> (Box<dyn Cmd>, bool, String) {
        match self {
            Command::List => (Box::new(cmd::list::List), false, String::new()),
            Command::Status => (Box::new(cmd::status::Status), false, String::new()),
            Command::Set { name } => (Box::new(cmd::set::Set(name.clone())), false, String::new()),
            Command::Switch { name, reboot } => {
                let target = name.clone();
                (Box::new(cmd::switch_cmd::Switch(name)), reboot, target)
            }
            Command::Cancel => (Box::new(cmd::cancel::Cancel), false, String::new()),
        }
    }
}
