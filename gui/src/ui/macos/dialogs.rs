use crate::rt::WinitWindow;
use crate::ui::FileType;
use std::path::PathBuf;

pub async fn open_file<T: WinitWindow>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Option<PathBuf> {
    todo!();
}

pub async fn open_dir<T: WinitWindow>(parent: &T, title: impl AsRef<str>) -> Option<PathBuf> {
    todo!()
}
