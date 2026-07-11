use std::process::Command;

use crate::boot::{BootEntry, BootManager};
use crate::error::BootError;

const EFI_GUID: &str = "{8be4df61-93ca-11d2-aa0d-00e098032b8c}";

// kernel32.dll
unsafe extern "system" {
    fn GetFirmwareEnvironmentVariableW(
        lpName: *const u16,
        lpGuid: *const u16,
        pBuffer: *mut u8,
        nSize: u32,
    ) -> u32;

    fn SetFirmwareEnvironmentVariableW(
        lpName: *const u16,
        lpGuid: *const u16,
        pValue: *const u8,
        nSize: u32,
    ) -> u32;

    fn GetLastError() -> u32;
}

pub struct WindowsBootManager {}

impl BootManager for WindowsBootManager {
    fn name(&self) -> &str {
        "WindowsBootManager"
    }

    fn entries(&self) -> Result<Vec<BootEntry>, BootError> {
        // 优先尝试直接读 NVRAM
        let entries = enumerate_via_firmware()?;
        if !entries.is_empty() {
            return Ok(entries);
        }

        // fallback: bcdedit /enum firmware
        enumerate_via_bcdedit()
    }

    fn set_next_boot(&self, entry: &BootEntry) -> Result<(), BootError> {
        let num = u16::from_str_radix(&entry.id, 16)
            .map_err(|_| BootError::BootEntryNotFound(format!("无效 id: {}", entry.id)))?;

        // 优先 SetFirmwareEnvironmentVariableW
        let value = num.to_le_bytes();
        let name_w = to_wide("BootNext");
        let guid_w = to_wide(EFI_GUID);

        let ret = unsafe {
            SetFirmwareEnvironmentVariableW(name_w.as_ptr(), guid_w.as_ptr(), value.as_ptr(), 2)
        };

        if ret != 0 {
            return Ok(());
        }

        // fallback: bcdedit /bootsequence
        let output = Command::new("bcdedit")
            .args(["/bootsequence", &entry.id])
            .output()
            .map_err(|e| BootError::BootNextWriteFailed(format!("bcdedit 执行失败: {e}")))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(BootError::BootNextWriteFailed(
                "SetFirmwareEnvironmentVariable 和 bcdedit 均失败".into(),
            ))
        }
    }

    fn clear_next_boot(&self) -> Result<(), BootError> {
        let name_w = to_wide("BootNext");
        let guid_w = to_wide(EFI_GUID);

        let ret = unsafe {
            SetFirmwareEnvironmentVariableW(name_w.as_ptr(), guid_w.as_ptr(), std::ptr::null(), 0)
        };

        if ret != 0 {
            return Ok(());
        }

        let err = unsafe { GetLastError() };
        if err != 203 {
            // try bcdedit fallback
            let _ = Command::new("bcdedit").args(["/bootsequence", ""]).output();
        }
        Ok(())
    }
}

fn enumerate_via_firmware() -> Result<Vec<BootEntry>, BootError> {
    let guid = to_wide(EFI_GUID);
    let mut entries = Vec::new();

    for boot_num in 0u16..=0xFF {
        let name = format!("Boot{:04X}", boot_num);
        let name_w = to_wide(&name);

        let mut buf = vec![0u8; 4096];
        let ret = unsafe {
            GetFirmwareEnvironmentVariableW(
                name_w.as_ptr(),
                guid.as_ptr(),
                buf.as_mut_ptr(),
                buf.len() as u32,
            )
        };

        if ret == 0 {
            continue;
        }

        buf.truncate(ret as usize);

        if buf.len() < 6 {
            continue;
        }

        let attrs = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        if attrs & 1 == 0 {
            continue;
        }

        if let Some(desc) = parse_description(&buf) {
            entries.push(BootEntry {
                id: format!("{:04X}", boot_num),
                description: desc,
            });
        }
    }

    Ok(entries)
}

fn enumerate_via_bcdedit() -> Result<Vec<BootEntry>, BootError> {
    let output = Command::new("bcdedit")
        .args(["/enum", "firmware"])
        .output()
        .map_err(|e| BootError::EfivarsInaccessible(format!("bcdedit 执行失败: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut entries = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_desc: Option<String> = None;

    for line in stdout.lines() {
        let line = line.trim();

        if line.starts_with("---") {
            if let (Some(id), Some(desc)) = (current_id.take(), current_desc.take()) {
                if id != "{fwbootmgr}" {
                    entries.push(BootEntry {
                        id,
                        description: desc,
                    });
                }
            }
            continue;
        }

        if let Some(rest) = strip_key(line, "identifier").or_else(|| strip_key(line, "标识符")) {
            current_id = Some(rest.to_string());
        }

        if let Some(rest) = strip_key(line, "description").or_else(|| strip_key(line, "描述")) {
            current_desc = Some(rest.to_string());
        }
    }

    if let (Some(id), Some(desc)) = (current_id, current_desc) {
        if id != "{fwbootmgr}" {
            entries.push(BootEntry {
                id,
                description: desc,
            });
        }
    }

    Ok(entries)
}

fn strip_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?;
    let rest = rest.strip_prefix(' ')?.trim();
    Some(rest)
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn parse_description(data: &[u8]) -> Option<String> {
    let u16s: Vec<u16> = data[6..]
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&w| w != 0)
        .collect();
    String::from_utf16(&u16s).ok()
}
