use anyhow::Context;
use clap::Parser;
use os_switch::boot::BootManager;
use os_switch::cli::{Cli, Command};
use os_switch::efi::LinuxEfiBootManager;
use os_switch::power;
use os_switch::privilege;

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("错误: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    privilege::ensure_root();

    match cli.command {
        Command::List => cmd_list(),
        Command::Status => cmd_status(),
        Command::Switch { name, reboot } => cmd_switch(&name, reboot),
        Command::Cancel => cmd_cancel(),
    }
}

fn cmd_list() -> anyhow::Result<()> {
    let mgr = LinuxEfiBootManager {};
    let entries = mgr.entries()?;
    for e in &entries {
        println!("  Boot{}  {}", e.id, e.description);
    }
    Ok(())
}

fn cmd_status() -> anyhow::Result<()> {
    let mgr = LinuxEfiBootManager {};
    let entries = mgr.entries()?;
    println!("  BootManager: {}", mgr.name());
    println!("  活跃引导项: {} 个", entries.len());

    match mgr.read_next_boot()? {
        Some(boot_num) => {
            let id = format!("{:04X}", boot_num);
            let desc = entries
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.description.as_str())
                .unwrap_or("<未知>");
            println!("  BootNext: Boot{} → {desc}", id);
        }
        None => println!("  BootNext: 未设置"),
    }

    Ok(())
}

fn cmd_switch(name: &str, reboot: bool) -> anyhow::Result<()> {
    let mgr = LinuxEfiBootManager {};
    let target = mgr
        .entries()?
        .into_iter()
        .find(|e| e.description == name)
        .with_context(|| format!("未找到引导项: {name}"))?;

    let mode = if reboot { "重启" } else { "休眠" };
    println!("即将{mode}切换到: {}", target.description);
    mgr.set_next_boot(&target).context("设置引导目标失败")?;
    println!("BootNext 已设置为: Boot{}", target.id);

    if reboot {
        power::reboot_now();
    } else {
        match power::hibernate_reboot() {
            Ok(()) => unreachable!(),
            Err(e) => {
                eprintln!("{e}");
                mgr.clear_next_boot().context("清除 BootNext 失败")?;
                eprintln!("BootNext 已清除，当前会话不受影响。");
            }
        }
    }

    Ok(())
}

fn cmd_cancel() -> anyhow::Result<()> {
    let mgr = LinuxEfiBootManager {};
    mgr.clear_next_boot().context("清除 BootNext 失败")?;
    println!("BootNext 已清除");
    Ok(())
}
