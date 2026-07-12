use crate::cmd::Output;

/// 终端输出层
pub fn render(output: &Output) {
    match output {
        Output::EntryList(entries) => {
            for e in entries {
                println!("  Boot{}  {}", e.id, e.description);
            }
        }
        Output::Status {
            manager_name,
            entry_count,
            boot_next,
        } => {
            println!("  BootManager: {manager_name}");
            println!("  活跃引导项: {entry_count} 个");
            match boot_next {
                Some((id, desc)) => println!("  BootNext: Boot{id} → {desc}"),
                None => println!("  BootNext: 未设置"),
            }
        }
        Output::SwitchResult {
            target,
            boot_next_id,
        } => {
            println!("BootNext 已设置为: Boot{boot_next_id} → {target}");
        }
        Output::CancelResult => {
            println!("BootNext 已清除");
        }
    }
}
