use crate::boot::BootManager;
use crate::cmd::{Cmd, Output};
use crate::error::BootError;

pub struct List;

impl Cmd for List {
    fn run(&self, mgr: &dyn BootManager) -> Result<Output, BootError> {
        let entries = mgr.entries()?;
        Ok(Output::EntryList(entries))
    }
}
