use clap::{Parser, Subcommand};

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
