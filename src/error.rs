use thiserror::Error;

#[derive(Error, Debug)]
pub enum BootError {
    #[error("efivars 不可访问: {0}")]
    EfivarsInaccessible(String),

    #[error("无法写入 BootNext: {0}")]
    BootNextWriteFailed(String),

    #[error("无法清除 BootNext: {0}")]
    BootNextClearFailed(String),

    #[error("休眠失败: {0}")]
    HibernateFailed(String),

    #[error("引导项未找到: {0}")]
    BootEntryNotFound(String),
}
