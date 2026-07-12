use anyhow::Context;
use clap::Parser;
use os_switch::boot::BootManager;
use os_switch::cli::Cli;
use os_switch::cmd::Output;
use os_switch::display;
use os_switch::efi::LinuxEfiBootManager;
use os_switch::power;
use os_switch::privilege;

fn main() {
    let cli = Cli::parse();

    if cli.command.needs_root() {
        privilege::ensure_root();
    }

    if let Err(e) = run(cli) {
        eprintln!("错误: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let needs_power = cli.command.needs_power();
    let (cmd, reboot, target_name) = cli.command.into_cmd();

    let mgr = LinuxEfiBootManager {};
    let output = cmd.run(&mgr).context("命令执行失败")?;
    display::render(&output);

    if needs_power {
        trigger_power(&mgr, reboot, &target_name)?;
    }

    Ok(())
}

fn trigger_power(mgr: &dyn BootManager, reboot: bool, name: &str) -> anyhow::Result<()> {
    let mode = if reboot { "重启" } else { "休眠" };
    println!("即将{mode}切换到: {name}");

    if reboot {
        power::reboot_system();
    } else {
        match power::try_hibernate(mgr) {
            Ok(()) => unreachable!(),
            Err(e) => {
                eprintln!("{e}");
                display::render(&Output::CancelResult);
                eprintln!("当前会话不受影响。");
            }
        }
    }

    Ok(())
}
