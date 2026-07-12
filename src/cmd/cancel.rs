use crate::boot::BootManager;
use crate::cmd::{Cmd, Output};
use crate::error::BootError;

pub struct Cancel;

impl Cmd for Cancel {
    fn run(&self, mgr: &dyn BootManager) -> Result<Output, BootError> {
        mgr.clear_next_boot()?;
        Ok(Output::CancelResult)
    }
}
