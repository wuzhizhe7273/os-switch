use crate::boot::BootManager;
use crate::cmd::{Cmd, Output};
use crate::error::BootError;

pub struct Set(pub String);

impl Cmd for Set {
    fn run(&self, mgr: &dyn BootManager) -> Result<Output, BootError> {
        let target = mgr
            .entries()?
            .into_iter()
            .find(|e| e.description == self.0)
            .ok_or_else(|| BootError::BootEntryNotFound(format!("未找到: {}", self.0)))?;

        let id = target.id.clone();
        let desc = target.description.clone();
        mgr.set_next_boot(&target)?;

        Ok(Output::SwitchResult {
            target: desc,
            boot_next_id: id,
        })
    }
}
