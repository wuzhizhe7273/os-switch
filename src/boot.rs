use crate::error::BootError;

pub struct BootEntry {
    pub id: String,
    pub description: String,
}

pub trait BootManager {
    fn name(&self) -> &str;
    fn entries(&self) -> Result<Vec<BootEntry>, BootError>;
    fn read_next_boot(&self) -> Result<Option<u16>, BootError>;
    fn set_next_boot(&self, entry: &BootEntry) -> Result<(), BootError>;
    fn clear_next_boot(&self) -> Result<(), BootError>;
}
