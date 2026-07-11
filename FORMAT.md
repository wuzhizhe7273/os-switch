# os-switch 数据格式参考

## 一、efivarfs 文件格式

每个 EFI 变量在 Linux 下对应一个文件：

```
/sys/firmware/efi/efivars/<变量名>-<vendor-guid>
```

### 1.1 文件二进制布局

```
offset  size  content
 0      4     attributes (u32 LE)
              bit 0: EFI_VARIABLE_NON_VOLATILE          (0x00000001) — 持久化到 NVRAM
              bit 1: EFI_VARIABLE_BOOTSERVICE_ACCESS    (0x00000002) — 引导服务阶段可访问
              bit 2: EFI_VARIABLE_RUNTIME_ACCESS        (0x00000004) — 运行时服务阶段可访问
              bit 3: EFI_VARIABLE_HARDWARE_ERROR_RECORD (0x00000008) — 硬件错误记录变量
              bit 4: EFI_VARIABLE_AUTHENTICATED_WRITE_ACCESS (0x00000010) — 需要签名验证才能写入（Secure Boot）
              bit 5: EFI_VARIABLE_RUNTIME_VOLATILE      (0x00000020) — 运行时创建，重启后消失
              bit 6: EFI_VARIABLE_TIME_BASED_AUTHENTICATED_WRITE_ACCESS (0x00000040) — 时间戳防回滚签名写入
              bit 7: EFI_VARIABLE_APPEND_WRITE          (0x00000080) — 追加写入而非覆盖
              bit 8~31: 保留

              引导变量标准值: 0x00000007 (bit 0|1|2 = NON_VOLATILE | BS | RT)
              BootCurrent:    0x00000006 (bit 1|2 = BS | RT，无 NON_VOLATILE)
              Secure Boot 下可能额外设置 bit 4 (0x17)
 4      k     variable_data — 格式取决于变量名，见下文
```

### 1.2 相关 GUID

```
8be4df61-93ca-11d2-aa0d-00e098032b8c  ← EFI_GLOBAL_VARIABLE（引导类变量通用）
```

同一 GUID 下的变量：

| 变量名 | 文件名示例 | data 内容 | 说明 |
|--------|-----------|-----------|------|
| `Boot####` | `Boot0003-8be4df61-...` | EFI_LOAD_OPTION | 单个引导项 (#### = 4 位 hex) |
| `BootOrder` | `BootOrder-8be4df61-...` | u16[] LE | 引导顺序 |
| `BootCurrent` | `BootCurrent-8be4df61-...` | u16 LE (2 字节) | 当前运行的引导项编号 |
| `BootNext` | `BootNext-8be4df61-...` | u16 LE (2 字节) | 一次性引导目标，开机后固件清除 |

---

## 二、EFI_LOAD_OPTION（Boot#### 的 data 部分）

### 2.1 二进制布局

```
offset  size  field
 0      4     Attributes (u32 LE)
              bit 0: LOAD_OPTION_ACTIVE   (0x00000001) — 引导项已激活
              bit 1: LOAD_OPTION_FORCE_RECONNECT (0x00000002) — 强制重连驱动
              bit 2: LOAD_OPTION_HIDDEN   (0x00000004) — 隐藏（不在引导菜单中显示）
              bit 3: LOAD_OPTION_CATEGORY (0x00000008) — UEFI 2.8+: 标记为分类条目
              bit 4: LOAD_OPTION_CATEGORY_APP (0x00000010) — UEFI 2.8+: 分类下的应用
              bit 5~31: 保留
              本程序只关心 bit 0：attrs & 1 != 0 保留，其余过滤掉
 4      2     FilePathListLength (u16 LE)
 6      n     Description (UTF-16LE, null-terminated)
 6+n    2     空终止符 \0x00\0x00
 6+n+2  m     FilePathList (m = FilePathListLength 字节)
 6+n+2+m k    OptionalData (剩余字节)
```

### 2.2 实测数据

取自本机 `/sys/firmware/efi/efivars/Boot0003-8be4df61-...`（Windows Boot Manager）：

```
偏移量  hex                                          解读
─────── ──────────────────────────────────────────── ────────────────
0x00    07 00 00 00                                   efivar attrs = 0x07
0x04    01 00 00 00  ←file attrs──┐                   LOAD_OPTION attrs = 0x01 (ACTIVE)
0x08    74 00          ←fpl_len──┐│                   FilePathListLength = 116
0x0A    57 00    W  ←description─┼┤                   逐个 ASCII 字符，
        .                        ││                   每个占 2 字节 (UTF-16LE)
        .                        ││                   总共 20 字符 = 40 字节
        .                        ││
0x32    72 00    r               ││
0x34    00 00    ←null term──────┼┘
0x36    04 01 2a 00 ...  ←fpl───┘                     FilePathList (116 字节)
```

拼起来：`57 00 69 00 6e 00 ... 72 00` → `W\x00i\x00n\x00...r\x00` → UTF-16LE 解码 → `"Windows Boot Manager"`

### 2.3 解析步骤

```
1. data[0..4]   → u32 LE  → LOAD_OPTION Attributes（bit 0 表示是否激活）
2. data[4..6]   → u16 LE  → FilePathListLength
3. data[6..]    → 逐 2 字节扫描 \0x00\0x00  → 定位 description 结束位置
4. description  → UTF-16LE → String::from_utf16()
5. 之后的内容   → 不需要（本程序不解析 FilePathList / OptionalData）
```

### 2.4 完整列表（本机实测）

```
Boot0000 → "EFI PXE 0 for IPv4 (E8-80-88-ED-78-84) "
Boot0001 → "EFI PXE 0 for IPv6 (E8-80-88-ED-78-84) "
Boot0003 → "Windows Boot Manager"
Boot0004 → "GRUB"
Boot2001 → "EFI USB Device"
Boot2002 → "EFI DVD/CDROM"
Boot2003 → "EFI Network"
```

---

## 三、BootCurrent / BootNext（data 部分格式）

内容为单个 u16 LE，2 字节。

### 3.1 实测数据

本机 `BootCurrent` 文件完整内容：

```
07 00 00 00  04 00    ← 共 6 字节
└── efivar attrs ─┘└── data: u16 LE = 0x0004 → Boot0004 (GRUB)
```

### 3.2 解析步骤

```
1. 读文件全部到 Vec<u8>
2. data[0..4] → 跳过 efivar attrs
3. data[4..6] → u16::from_le_bytes() → 编号
```

---

## 四、BootOrder（data 部分格式）

内容为 u16 LE 序列，每个 2 字节。

### 4.1 实测数据

本机 `BootOrder` 文件完整内容：

```
07 00 00 00  ← efivar attrs
04 00          ← Boot0004 (GRUB)
01 20          ← Boot2001 (USB)
03 00          ← Boot0003 (Windows)
02 20          ← Boot2002 (DVD)
03 20          ← Boot2003 (Network)
```

总 data 长度 = 5 × 2 = 10 字节。

### 4.2 解析步骤

```
1. data[0..4] → 跳过 efivar attrs
2. data[4..]  → 每 2 字节一组 → u16::from_le_bytes()
```

---

## 五、文件名提取 boot number

**变量名格式**：`BootXXXX-{guid}`

**提取规则**：

```
1. name 以 "Boot" 开头
2. name 总长 >= 9
3. name[4..8] 为 4 位十六进制数字 (0-9, A-F, a-f)
4. 转换为 u16
```

**伪代码**：

```rust
fn parse_boot_num(filename: &str) -> Option<u16> {
    if !filename.starts_with("Boot") || filename.len() < 9 { return None; }
    u16::from_str_radix(&filename[4..8], 16).ok()
}
```

**自动排除的变量**：`BootCurrent`、`BootOrder`、`BootNext` — 它们 [4..8] 是 `Curr`、`Orde`、`Next` 不是 hex 数字，`from_str_radix` 返回 `Err`。

---

## 六、GRUB 配置文件格式（Legacy BIOS 路径）

### 6.1 grub.cfg — 引导项定义

**路径**（按优先级）：
- `/boot/grub/grub.cfg`
- `/boot/grub2/grub.cfg`

**格式**：

```
menuentry 'Windows Boot Manager (on /dev/nvme0n1p1)' --class windows ... {
        ...
}
menuentry "Arch Linux" --class arch ... {
        ...
}
menuentry Ubuntu {
        ...
}
submenu 'Advanced options' {    ← 忽略或展开
        menuentry '...'
}
```

**提取规则**：

1. 逐行扫描，只处理 `menuentry` 开头的行
2. 提取第一个引号字符串作为 title：
   - 单引号：`'...'`
   - 双引号：`"..."`
   - 无引号：取第一个空格前的单词
3. 忽略 submenu 内嵌套的 menuentry（可选：也可以平铺展开）

### 6.2 grubenv — 一次性引导设置

**路径**：
- `/boot/grub/grubenv`
- `/boot/grub2/grubenv`

**格式**：

```
# GRUB Environment Block
saved_entry=Windows Boot Manager (on /dev/nvme0n1p1)
next_entry=
####################################################################################################################
```

**规则**：

- 固定长度文件（通常 1024 字节）
- 开头 `# GRUB Environment Block\n`
- 中间是普通 `key=value` 行
- 尾部用 `#` 填充到固定长度
- `saved_entry` — 配合 `grub_default=saved` 的配置使用，设置下次引导的菜单项

**写入方式**（二选一）：

方案 A：直接文件操作 — 读原文件 → 替换 `saved_entry=...` 行 → 保持原长度写回
方案 B：`grub-editenv set saved_entry=...` — 调用 GRUB 自带工具

---

## 七、本程序需要的关键信息汇总

| 需要什么 | 从哪里来 | 怎么读 |
|---------|---------|--------|
| 所有可用引导项列表 | Boot#### efivars / grub.cfg | UEFI: 枚举 efivars 目录 + 解析 EFI_LOAD_OPTION<br>GRUB: 读 grub.cfg + 正则 `menuentry` |
| 当前运行的引导项 | BootCurrent efivar | data[4..6] → u16 LE |
| 当前是否设了 BootNext | BootNext efivar | data[4..6] → u16 LE |
| 设置一次性引导目标 | 写 BootNext efivar / grubenv | UEFI: `[0x07, 0, 0, 0, lo, hi]` → 文件<br>GRUB: 写 `saved_entry=title` |
| 取消一次性引导 | 删 BootNext efivar / 清 grubenv | 删除文件 / `grub-editenv unset` |

---

## 八、检查清单

编写 efiboot.rs / grubboot.rs 时逐项验证：

- [ ] 读取 efivars 文件时跳过前 4 字节 attributes
- [ ] UTF-16LE 解码 description 时步长为 2，跳过空终止符
- [ ] 只保留 LOAD_OPTION_ACTIVE 为 1 的条目（过滤掉禁用的引导项）
- [ ] Boot#### 文件名匹配只取前 8 位（`BootXXXX`），之后的 `-` 属于 GUID 分隔符
- [ ] 编号 0x0000 是合法值（PXE IPv4 就是 0x0000）
- [ ] 写 BootNext 时要确保 6 字节完整（4 attrs + 2 data）
