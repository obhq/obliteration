use super::FileType;
use slint::ComponentHandle;
use std::path::PathBuf;

pub async fn open_file<T: ComponentHandle>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Option<PathBuf> {
    todo!();
}
