# EFI 变量格式参考

## efivarfs 文件格式

每个 EFI 变量在 Linux 下对应一个文件：

```
/sys/firmware/efi/efivars/<变量名>-<vendor-guid>
```

### 文件二进制布局

```
offset  size  content
 0      4     attributes (u32 LE)
              bit 0: EFI_VARIABLE_NON_VOLATILE          (0x00000001) — 持久化到 NVRAM
              bit 1: EFI_VARIABLE_BOOTSERVICE_ACCESS    (0x00000002) — 引导服务阶段可访问
              bit 2: EFI_VARIABLE_RUNTIME_ACCESS        (0x00000004) — 运行时服务阶段可访问
              bit 3: EFI_VARIABLE_HARDWARE_ERROR_RECORD (0x00000008)
              bit 4: EFI_VARIABLE_AUTHENTICATED_WRITE_ACCESS (0x00000010) — 需签名验证
              bit 5: EFI_VARIABLE_RUNTIME_VOLATILE      (0x00000020)
              bit 6: EFI_VARIABLE_TIME_BASED_AUTHENTICATED_WRITE_ACCESS (0x00000040)
              bit 7: EFI_VARIABLE_APPEND_WRITE          (0x00000080)

              引导变量标准值: 0x00000007 (bit 0|1|2)
              BootCurrent:    0x00000006 (无 NON_VOLATILE)
 4      k     variable_data — 格式取决于变量名
```

### 引导变量 GUID

```
8be4df61-93ca-11d2-aa0d-00e098032b8c  ← EFI_GLOBAL_VARIABLE
```

同一 GUID 下的变量：

| 变量名 | data 内容 | 说明 |
|--------|-----------|------|
| `Boot####` | EFI_LOAD_OPTION | 单个引导项 (#### = 4 位 hex) |
| `BootOrder` | u16[] LE | 引导顺序 |
| `BootCurrent` | u16 LE (2 字节) | 当前运行的引导项编号 |
| `BootNext` | u16 LE (2 字节) | 一次性引导目标，开机后固件清除 |

---

## EFI_LOAD_OPTION（Boot#### 的 data 部分）

### 二进制布局

```
offset  size  field
 0      4     Attributes (u32 LE)
              bit 0: LOAD_OPTION_ACTIVE   (0x00000001) — 引导项已激活
              bit 1: LOAD_OPTION_FORCE_RECONNECT (0x00000002)
              bit 2: LOAD_OPTION_HIDDEN   (0x00000004) — 隐藏
              bit 3: LOAD_OPTION_CATEGORY (0x00000008) — UEFI 2.8+
              bit 4: LOAD_OPTION_CATEGORY_APP (0x00000010) — UEFI 2.8+
 4      2     FilePathListLength (u16 LE)
 6      n     Description (UTF-16LE, null-terminated)
 6+n    2     空终止符 \0x00\0x00
 6+n+2  m     FilePathList (m = FilePathListLength 字节)
 6+n+2+m k    OptionalData (剩余字节)
```

### 实测数据

```
Boot0003 → "Windows Boot Manager"
Boot0004 → "GRUB"
Boot2001 → "EFI USB Device"
```

**Hex 示例**（Boot0003，Windows Boot Manager）：

```
偏移量  hex                                          解读
─────── ──────────────────────────────────────────── ────────────────
0x00    07 00 00 00                                   efivar attrs = 0x07
0x04    01 00 00 00  ←file attrs──┐                   LOAD_OPTION attrs = 0x01 (ACTIVE)
0x08    74 00          ←fpl_len──┐│                   FilePathListLength = 116
0x0A    57 00    W  ←description─┼┤                   UTF-16LE: "Windows Boot Manager"
        .                        ││                   20 字符 = 40 字节
0x32    72 00    r               ││
0x34    00 00    ←null term──────┼┘
0x36    04 01 2a 00 ...  ←fpl───┘                     FilePathList (116 字节)
```

### Rust 解析

```rust
fn parse_description(load_option: &[u8]) -> Option<String> {
    let u16s: Vec<u16> = load_option[6..]
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&w| w != 0)
        .collect();
    String::from_utf16(&u16s).ok()
}
```

---

## BootCurrent / BootNext

单个 u16 LE，2 字节。

```
BootCurrent 文件完整内容:
07 00 00 00  04 00    ← efivar attrs + u16 LE = 0x0004 → Boot0004
```

---

## BootOrder

u16 LE 序列，每个 2 字节。

```
07 00 00 00  ← efivar attrs
04 00          ← Boot0004
01 20          ← Boot2001
03 00          ← Boot0003
02 20          ← Boot2002
03 20          ← Boot2003
```

---

## 文件名提取 boot number

```rust
fn parse_boot_var_name(name: &str) -> Option<u16> {
    if name.len() < 9 || !name.starts_with("Boot") {
        return None;
    }
    u16::from_str_radix(&name[4..8], 16).ok()
}
```

`BootCurrent`、`BootOrder`、`BootNext` 自动被过滤：[4..8] 是 `Curr`、`Orde`、`Next`，不是 hex。
