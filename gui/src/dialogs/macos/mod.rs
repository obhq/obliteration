use super::FileType;
use slint::ComponentHandle;

pub async fn open_file<T: ComponentHandle>(parent: &T, title: impl AsRef<str>, ty: FileType) {
    todo!();
}
