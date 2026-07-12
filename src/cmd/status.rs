use crate::boot::BootManager;
use crate::cmd::{Cmd, Output};
use crate::error::BootError;

pub struct Status;

impl Cmd for Status {
    fn run(&self, mgr: &dyn BootManager) -> Result<Output, BootError> {
        let entries = mgr.entries()?;
        let boot_next = match mgr.read_next_boot()? {
            Some(num) => {
                let id = format!("{:04X}", num);
                let desc = entries
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.description.clone())
                    .unwrap_or_else(|| "<未知>".into());
                Some((id, desc))
            }
            None => None,
        };

        Ok(Output::Status {
            manager_name: mgr.name().into(),
            entry_count: entries.len(),
            boot_next,
        })
    }
}
