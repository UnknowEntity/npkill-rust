use std::path::PathBuf;

use anyhow::Result;

pub enum HandlerEvent {
    FindDir(PathBuf),
    FinishedFinding,
    FileSize(usize, Result<u64>),
    FileDeleted(usize, Result<()>),
}
