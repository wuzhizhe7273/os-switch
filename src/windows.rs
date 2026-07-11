use crate::boot::{BootEntry, BootManager};
use crate::error::BootError;

const EFI_GUID: &str = "{8be4df61-93ca-11d2-aa0d-00e098032b8c}";

// kernel32.dll
extern "system" {
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
                continue; // variable doesn't exist
            }

            buf.truncate(ret as usize);

            // EFI_LOAD_OPTION 解析：跳过 Attributes(4) + FilePathListLength(2)，从 offset 6 取 description
            if buf.len() < 6 {
                continue;
            }

            let attrs = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
            if attrs & 1 == 0 {
                continue; // LOAD_OPTION_ACTIVE 未设置
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

    fn set_next_boot(&self, entry: &BootEntry) -> Result<(), BootError> {
        let num = u16::from_str_radix(&entry.id, 16)
            .map_err(|_| BootError::BootEntryNotFound(format!("无效 id: {}", entry.id)))?;

        let value = num.to_le_bytes();
        let name_w = to_wide("BootNext");
        let guid_w = to_wide(EFI_GUID);

        let ret = unsafe {
            SetFirmwareEnvironmentVariableW(name_w.as_ptr(), guid_w.as_ptr(), value.as_ptr(), 2)
        };

        if ret == 0 {
            let err = unsafe { GetLastError() };
            Err(BootError::BootNextWriteFailed(format!(
                "SetFirmwareEnvironmentVariable 失败 (错误码: {err})"
            )))
        } else {
            Ok(())
        }
    }

    fn clear_next_boot(&self) -> Result<(), BootError> {
        let name_w = to_wide("BootNext");
        let guid_w = to_wide(EFI_GUID);

        let ret = unsafe {
            SetFirmwareEnvironmentVariableW(name_w.as_ptr(), guid_w.as_ptr(), std::ptr::null(), 0)
        };

        if ret == 0 {
            let err = unsafe { GetLastError() };
            // ERROR_ENVVAR_NOT_FOUND (203) = 没有 BootNext，不算错误
            if err != 203 {
                return Err(BootError::BootNextClearFailed(format!(
                    "SetFirmwareEnvironmentVariable 失败 (错误码: {err})"
                )));
            }
        }

        Ok(())
    }
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
