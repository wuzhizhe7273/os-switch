use std::fs;

use crate::boot::{BootEntry, BootManager};
use crate::error::BootError;

const EFIVARS_DIR: &str = "/sys/firmware/efi/efivars";
const EFI_GUID: &str = "8be4df61-93ca-11d2-aa0d-00e098032b8c";

pub struct LinuxEfiBootManager {}

impl BootManager for LinuxEfiBootManager {
    fn name(&self) -> &str {
        "EfiBootManager"
    }

    fn entries(&self) -> Result<Vec<BootEntry>, BootError> {
        let dir = fs::read_dir(EFIVARS_DIR)
            .map_err(|e| BootError::EfivarsInaccessible(format!("无法读取 efivars 目录: {e}")))?;

        let entries: Vec<BootEntry> = dir
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let os_name = entry.file_name();
                let name = os_name.to_string_lossy();
                parse_boot_var_name(&name).map(|num| (num, entry))
            })
            .filter_map(|(num, entry)| {
                let raw = fs::read(entry.path()).ok()?;
                if raw.len() < 10 {
                    return None;
                }
                Some((num, raw))
            })
            .map(|(num, raw)| (num, raw[4..].to_vec()))
            .filter(|(_, option)| {
                let attrs = u32::from_le_bytes([option[0], option[1], option[2], option[3]]);
                attrs & 1 != 0
            })
            .filter_map(|(num, option)| {
                parse_description(&option).map(|description| BootEntry {
                    id: format!("{:04X}", num),
                    description,
                })
            })
            .collect();

        Ok(entries)
    }

    fn set_next_boot(&self, entry: &BootEntry) -> Result<(), BootError> {
        let num = u16::from_str_radix(&entry.id, 16)
            .map_err(|_| BootError::BootEntryNotFound(format!("无效 id: {}", entry.id)))?;
        let [lo, hi] = num.to_le_bytes();
        let buf: [u8; 6] = [0x07, 0x00, 0x00, 0x00, lo, hi];
        let path = format!("{}/BootNext-{}", EFIVARS_DIR, EFI_GUID);
        fs::write(&path, &buf).map_err(|e| BootError::BootNextWriteFailed(format!("{e}")))?;
        Ok(())
    }

    fn clear_next_boot(&self) -> Result<(), BootError> {
        let path = format!("{}/BootNext-{}", EFIVARS_DIR, EFI_GUID);
        fs::remove_file(&path).or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(())
            } else {
                Err(BootError::BootNextClearFailed(format!("{e}")))
            }
        })
    }
}

fn parse_boot_var_name(name: &str) -> Option<u16> {
    if name.len() < 9 || !name.starts_with("Boot") {
        return None;
    }
    u16::from_str_radix(&name[4..8], 16).ok()
}

fn parse_description(load_option: &[u8]) -> Option<String> {
    let u16s: Vec<u16> = load_option[6..]
        .chunks(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&w| w != 0)
        .collect();
    String::from_utf16(&u16s).ok()
}
