use crate::ui::{DesktopWindow, FileType};
use std::path::PathBuf;

pub async fn open_file<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Option<PathBuf> {
    todo!();
}

pub async fn open_dir<T: DesktopWindow>(parent: &T, title: impl AsRef<str>) -> Option<PathBuf> {
    todo!()
}
