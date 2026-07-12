# os-switch

Linux 双系统快速切换工具。支持休眠切换（保存会话）和直接重启切换。

## 安装

### Arch Linux

```bash
./scripts/makepkg.sh
```
### 从源码编译

```bash
cargo build --release
sudo install -m755 target/release/os-switch /usr/local/bin/
```

## 用法

```bash
os-switch list                        # 列出所有可用引导项
os-switch status                      # 显示当前状态
os-switch set <name>                  # 设置 BootNext（不触发休眠）
os-switch switch <name>               # 休眠切换到目标系统
os-switch switch --reboot <name>      # 直接重启切换
os-switch cancel                      # 清除 BootNext
```

## 原理

通过设置 UEFI `BootNext` 变量告诉固件下次开机直接启动目标 OS，然后触发休眠或重启。

```
Linux → set BootNext=0003(Windows) → hibernate → 断电
                                                     ↓
                                              按电源键 → Windows
```

详细格式说明见 [doc/efivars.md](doc/efivars.md)。

## 构建要求

- Rust 1.70+
- systemd（用于触发休眠）

## 已知限制

- **Lenovo Legion 系列**：Windows 重启后 Linux 启动可能循环崩溃，这是 Lenovo 固件 bug，需从 Windows 关机而非重启返回 Linux
- 休眠需要 swap 分区，且休眠恢复依赖 `systemctl hibernate`

## License

MIT
