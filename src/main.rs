use anyhow::Context;
use clap::Parser;
use os_switch::boot::BootManager;
use os_switch::cli::{Cli, Command};
use os_switch::cmd::{self, Cmd, Output};
use os_switch::display;
use os_switch::efi::LinuxEfiBootManager;
use os_switch::power;
use os_switch::privilege;

fn main() {
    let cli = Cli::parse();

    // switch/cancel 需要 root 写 efivars
    match &cli.command {
        Command::Switch { .. } | Command::Cancel => privilege::ensure_root(),
        _ => {}
    }

    if let Err(e) = run(cli) {
        eprintln!("错误: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mgr = LinuxEfiBootManager {};
    let (output, reboot, target_name) = execute_command(&cli, &mgr)?;
    display::render(&output);

    if let Output::SwitchResult { .. } = &output {
        trigger_power(&mgr, reboot, &target_name)?;
    }

    Ok(())
}

fn execute_command(cli: &Cli, mgr: &dyn BootManager) -> anyhow::Result<(Output, bool, String)> {
    let (cmd, reboot, target): (Box<dyn Cmd>, bool, String) = match &cli.command {
        Command::List => (Box::new(cmd::list::List), false, String::new()),
        Command::Status => (Box::new(cmd::status::Status), false, String::new()),
        Command::Switch { name, reboot } => (
            Box::new(cmd::switch_cmd::Switch(name.clone())),
            *reboot,
            name.clone(),
        ),
        Command::Cancel => (Box::new(cmd::cancel::Cancel), false, String::new()),
    };

    let output = cmd.run(mgr).context("命令执行失败")?;
    Ok((output, reboot, target))
}

fn trigger_power(mgr: &dyn BootManager, reboot: bool, name: &str) -> anyhow::Result<()> {
    let mode = if reboot { "重启" } else { "休眠" };
    println!("即将{mode}切换到: {name}");

    if reboot {
        power::reboot_now();
    } else {
        match power::hibernate_reboot() {
            Ok(()) => unreachable!(),
            Err(e) => {
                eprintln!("{e}");
                mgr.clear_next_boot().context("清除 BootNext 失败")?;
                display::render(&Output::CancelResult);
                eprintln!("当前会话不受影响。");
            }
        }
    }

    Ok(())
}
