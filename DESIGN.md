# os-switch 设计文档（v1: Linux → Windows）

## 概述

Linux 命令行工具，用于快速切换到 Windows 双系统。支持两种模式：

- **休眠切换**：Linux 休眠，开机后直接进 Windows
- **重启切换**：直接重启进 Windows

当前 v1 仅实现 Linux → Windows。Windows → Linux 留待 v2。

## 核心原则

1. 零外部 crate 依赖
2. 自动扫描 + 配置文件覆盖
3. 合法自动提权（sudo）
4. 最小化系统修改（仅写一次性 BootNext / grubenv）

---

## 项目结构

```
src/
├── main.rs         # CLI 入口：参数解析、业务流程编排
├── boot.rs         # trait BootManager + 共享类型（BootEntry, BootStatus）
├── efiboot.rs      # impl BootManager：UEFI + efivars 路径
├── grubboot.rs     # impl BootManager：GRUB + grubenv 路径
├── power.rs        # 系统操作：hibernate() / reboot()
├── config.rs       # 配置文件：~/.config/os-switch/config.toml
└── privilege.rs    # 提权：euid 检测 + sudo execvp
```

**依赖关系**：

```
main.rs ──────────┬── boot.rs (trait + 类型)
                  ├── config.rs
                  ├── privilege.rs
                  └── power.rs

efiboot.rs ─────── boot.rs
grubboot.rs ────── boot.rs
```

- `boot.rs` 不依赖任何其他模块，只定义接口
- `efiboot.rs` / `grubboot.rs` 各自独立实现同一个 trait
- `main.rs` 根据 `/sys/firmware/efi` 的存在选择实例化哪个实现

---

## CLI 接口

```
os-switch list                       # 列出所有引导项
os-switch status                     # 当前系统状态 + efivars 可写性
os-switch switch [--reboot] <name>   # 切换（默认休眠）
os-switch cancel                     # 清除 BootNext
```

- `<name>` 可以是自动发现的 description，也可以是配置别名

---

## 模块设计

### 1. `main.rs`

职责单一：组装 + 编排，不含任何平台细节。

```rust
fn main() {
    ensure_root_or_elevate();           // privilege.rs
    let config = Config::load();        // config.rs

    match parse_command(&args) {
        Command::List => {
            let mgr = create_boot_manager();
            let entries = mgr.list_entries()?;
            print_entries(&entries, &config.aliases);
        }
        Command::Status => {
            let mgr = create_boot_manager();
            let status = mgr.check_status()?;
            print_status(&status);
        }
        Command::Switch { name, mode } => {
            let mgr = create_boot_manager();
            let entries = mgr.list_entries()?;
            let target = resolve(&entries, &config.aliases, &name)?;

            confirm_switch(&target, mode)?;
            mgr.set_next_boot(&target)?;

            match mode {
                Mode::Hibernate => power::hibernate(),
                Mode::Reboot    => power::reboot(),
            }
        }
        Command::Cancel => {
            let mgr = create_boot_manager();
            mgr.clear_next_boot()?;
            println!("BootNext 已清除");
        }
    }
}
```

**`create_boot_manager()` 工厂函数**：

```rust
fn create_boot_manager() -> Box<dyn BootManager> {
    if Path::new("/sys/firmware/efi").exists() {
        Box::new(EfiBootManager::new())
    } else {
        Box::new(GrubBootManager::new())
    }
}
```

**名称解析逻辑**（`resolve` 函数）：

```
1. config.aliases.get(name) → 匹配 description
2. entries 中 description 模糊匹配（大小写不敏感、包含匹配）
3. 都未命中 → 报错
```

---

### 2. `boot.rs` — trait + 共享类型

```rust
// ─── 共享类型 ───

pub struct BootEntry {
    pub id: String,           // 管理器内部标识，set_next_boot 时用它回查
    pub description: String,  // 显示名称，如 "Windows Boot Manager"
}

pub struct BootStatus {
    pub boot_mode: String,          // "UEFI" | "Legacy (GRUB)"
    pub is_uefi: bool,
    pub efivars_writable: bool,
    pub efivars_issues: Vec<String>,
    pub has_boot_next: bool,
    pub boot_next_description: Option<String>,
}

// ─── trait ───

pub trait BootManager {
    /// 管理器名称（调试/状态输出用）
    fn name(&self) -> &str;

    /// 枚举所有可引导的操作系统条目
    fn list_entries(&self) -> Result<Vec<BootEntry>, String>;

    /// 设置一次性引导目标
    fn set_next_boot(&self, entry: &BootEntry) -> Result<(), String>;

    /// 清除已设置的一次性引导
    fn clear_next_boot(&self) -> Result<(), String>;

    /// 系统状态检测
    fn check_status(&self) -> Result<BootStatus, String>;
}
```

**为什么 `BootEntry.id` 用 `String` 而不是 `u16`？** UEFI 用 boot number（可转为 16 进制字符串 "0003"），GRUB 用 menuentry 标题字符串。`String` 统一两种路径，实现在各自模块内部 parse。

---

### 3. `efiboot.rs` — UEFI / efivars 实现

实现 `BootManager` trait。

#### 3.1 内部数据结构

```rust
pub struct EfiBootManager {
    efivars_dir: PathBuf,
    guid: String,
}

/// EFI_LOAD_OPTION 解析结果
struct EfiLoadOptionData {
    attributes: u32,
    description: String,
}
```

#### 3.2 `list_entries()` 实现

```
1. 读取 /sys/firmware/efi/efivars/ 目录
2. 筛选文件名: BootXXXX-{guid}（XXXX = 4 位 hex，跳过 BootCurrent/BootOrder/BootNext）
3. 对每个文件:
   a. 读取全部字节
   b. 跳过 [0..4]（efivar attributes）
   c. 解析 [4..]: EFI_LOAD_OPTION → 提取 Attributes / Description
   d. 只保留 LOAD_OPTION_ACTIVE (bit 0) 为 1 的条目
   e. BootEntry { id: "XXXX", description: "..." }
```

#### 3.3 `set_next_boot()` 实现

```
1. 将 entry.id 解析为 u16（from_str_radix(16)）
2. 构造 6 字节: [0x07, 0, 0, 0, lo, hi]
3. 写入 /sys/firmware/efi/efivars/BootNext-{guid}
```

#### 3.4 `clear_next_boot()` 实现

```
删除 /sys/firmware/efi/efivars/BootNext-{guid}
文件不存在时不报错
```

#### 3.5 `check_status()` 实现

```
1. is_uefi = true（自身就是 UEFI 管理器）
2. 尝试读写测试变量检测 efivars_writable
3. 读取 BootNext 是否存在，存在则查找对应 description
4. 记录 efivars 不可写的原因
```

#### 3.6 EFI_LOAD_OPTION 解析

```rust
fn parse_load_option(data: &[u8]) -> Result<EfiLoadOptionData, String> {
    if data.len() < 6 { return Err("too short".into()); }

    let attrs = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let _fpl_len = u16::from_le_bytes([data[4], data[5]]) as usize;

    // 从 offset 6 开始，步长 2，搜索 \0x00\0x00
    let start = 6;
    let mut end = start;
    while end + 1 < data.len() {
        if data[end] == 0 && data[end + 1] == 0 { break; }
        end += 2;
    }
    if end + 1 >= data.len() { return Err("no null terminator".into()); }

    let mut u16s = Vec::with_capacity((end - start) / 2);
    let mut i = start;
    while i < end {
        u16s.push(u16::from_le_bytes([data[i], data[i + 1]]));
        i += 2;
    }
    Ok(EfiLoadOptionData {
        attributes: attrs,
        description: String::from_utf16(&u16s)
            .map_err(|_| "invalid utf-16".into())?,
    })
}
```

---

### 4. `grubboot.rs` — GRUB / grubenv 实现

实现 `BootManager` trait。

#### 4.1 内部数据结构

```rust
pub struct GrubBootManager {
    grub_cfg_paths: Vec<PathBuf>,
    grubenv_paths: Vec<PathBuf>,
}
```

#### 4.2 `list_entries()` 实现

```
1. 依次尝试 grub_cfg_paths，找到第一个存在的文件
2. 逐行扫描，匹配 menuentry 'Title' / "Title" / Title 格式
3. BootEntry { id: title, description: title }
```

**menuentry 解析器**：

```rust
fn parse_grub_entries(content: &str) -> Vec<String> {
    let mut entries = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("menuentry ") { continue; }

        let rest = &trimmed["menuentry ".len()..];
        let title = extract_quoted_string(rest).unwrap_or_default();
        if !title.is_empty() {
            entries.push(title.to_string());
        }
    }
    entries
}

fn extract_quoted_string(s: &str) -> Option<&str> {
    if s.starts_with('\'') {
        let end = s[1..].find('\'')?;
        Some(&s[1..end + 1])
    } else if s.starts_with('"') {
        let end = s[1..].find('"')?;
        Some(&s[1..end + 1])
    } else {
        s.split_whitespace().next()
    }
}
```

#### 4.3 `set_next_boot()` 实现

```
方案 A（优先）: 调用 /usr/sbin/grub-editenv set saved_entry=<entry.id>
方案 B（备选）: 直接读写 grubenv 文件，替换/新增 saved_entry 行
```

#### 4.4 `clear_next_boot()` 实现

```
grub-editenv unset saved_entry
```

#### 4.5 `check_status()` 实现

```
is_uefi = false
efivars_writable = false
读取 grubenv 检查 saved_entry 是否存在
```

---

### 5. `power.rs` — 系统操作

独立的纯函数模块，不依赖任何其他模块。

```rust
extern "C" {
    fn sync();
    fn reboot(cmd: i32) -> i32;
}
const LINUX_REBOOT_CMD_RESTART: i32 = 0x01234567;

/// 休眠系统（不返回）
pub fn hibernate() -> ! {
    unsafe { sync(); }

    // 方案 A: systemctl hibernate
    let result = std::process::Command::new("systemctl")
        .args(["hibernate"])
        .spawn();

    // 方案 B: 直接写 /sys/power/state（fallback）
    if result.is_err() {
        let _ = std::fs::write("/sys/power/state", "disk");
    }

    eprintln!("休眠失败");
    std::process::exit(1);
}

/// 重启系统（不返回）
pub fn reboot() -> ! {
    unsafe { sync(); }
    unsafe { reboot(LINUX_REBOOT_CMD_RESTART); }
    eprintln!("重启失败");
    std::process::exit(1);
}
```

---

### 6. `config.rs` — 配置文件

路径：`$XDG_CONFIG_HOME/os-switch/config.toml`（默认 `~/.config/os-switch/config.toml`）

```rust
pub struct Config {
    pub aliases: HashMap<String, String>,   // 短名 → description
    pub default_mode: SwitchMode,
}

pub enum SwitchMode { Hibernate, Reboot }
```

**TOML 格式**：

```toml
[aliases]
win = "Windows Boot Manager"

[defaults]
mode = "hibernate"
```

**解析器**：手写最小实现，支持 `[section]`、`key = "value"`、`# 注释`。

首次运行时自动创建模板文件。

---

### 7. `privilege.rs` — 提权

```rust
extern "C" {
    fn geteuid() -> u32;
    fn execvp(file: *const i8, argv: *const *const i8) -> i32;
}

pub fn is_root() -> bool {
    unsafe { geteuid() == 0 }
}

pub fn ensure_root() {
    if is_root() { return; }
    elevate();
}

fn elevate() -> ! {
    let args: Vec<CString> = std::env::args()
        .map(|a| CString::new(a).unwrap())
        .collect();

    let sudo = CString::new("sudo").unwrap();
    let mut argv: Vec<*const i8> = vec![sudo.as_ptr()];
    for a in &args { argv.push(a.as_ptr()); }
    argv.push(std::ptr::null());

    unsafe { execvp(sudo.as_ptr(), argv.as_ptr()); }
    eprintln!("提权失败");
    std::process::exit(1);
}
```

---

## 执行流程

```
$ os-switch switch win
    │
    ├── privilege::ensure_root()           → geteuid() != 0 → execvp("sudo", ...)
    ├── config::Config::load()             → aliases["win"] = "Windows Boot Manager"
    ├── create_boot_manager()
    │       → /sys/firmware/efi 存在? → EfiBootManager
    ├── mgr.list_entries()
    │       → [BootEntry { id="0003", desc="Windows Boot Manager" }, ...]
    ├── resolve(&entries, &aliases, "win")
    │       → BootEntry { id="0003", desc="Windows Boot Manager" }
    ├── confirm_switch()                   → 打印确认信息, [y/N]
    ├── mgr.set_next_boot(&entry)
    │       → EfiBootManager: 写 BootNext=0003
    └── power::hibernate()                 → systemctl hibernate
```

---

## 构建

```toml
[package]
name = "os-switch"
version = "0.1.0"
edition = "2021"

[dependencies]
# 空

[profile.release]
opt-level = "s"
lto = true
strip = true
```
