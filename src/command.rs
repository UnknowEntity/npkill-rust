use std::path::PathBuf;

pub enum HandlerCommand {
    FindNodeModules(PathBuf),
    GetDirSize(usize, PathBuf),
    DeleteDir(usize, PathBuf),
    Quit,
}
