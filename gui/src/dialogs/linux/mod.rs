use super::FileType;
use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use ashpd::desktop::ResponseError;
use ashpd::WindowIdentifier;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use slint::ComponentHandle;
use std::future::Future;
use std::path::PathBuf;

pub async fn open_file<T: ComponentHandle>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Option<PathBuf> {
    with_window_id(parent, move |parent| async move {
        // Build filter.
        let filter = match ty {
            FileType::Firmware => FileFilter::new("Firmware Dump").glob("*.obf"),
        };

        // Send the request
        let resp = match SelectedFiles::open_file()
            .identifier(parent)
            .title(title.as_ref())
            .modal(true)
            .filter(filter)
            .send()
            .await
            .unwrap()
            .response()
        {
            Ok(v) => v,
            Err(ashpd::Error::Response(ResponseError::Cancelled)) => return None,
            Err(_) => unimplemented!(),
        };

        // Get file path.
        Some(resp.uris().first().unwrap().to_file_path().unwrap())
    })
    .await
}

pub async fn open_dir<T: ComponentHandle>(parent: &T, title: impl AsRef<str>) -> Option<PathBuf> {
    with_window_id(parent, move |parent| async move {
        // Send the request
        let resp = match SelectedFiles::open_file()
            .identifier(parent)
            .title(title.as_ref())
            .modal(true)
            .directory(true)
            .send()
            .await
            .unwrap()
            .response()
        {
            Ok(v) => v,
            Err(ashpd::Error::Response(ResponseError::Cancelled)) => return None,
            Err(_) => unimplemented!(),
        };

        // Get directory path.
        Some(resp.uris().first().unwrap().to_file_path().unwrap())
    })
    .await
}

async fn with_window_id<R, P, F>(parent: &P, f: F) -> R::Output
where
    R: Future,
    P: ComponentHandle,
    F: FnOnce(Option<WindowIdentifier>) -> R,
{
    // Get display handle. All local variable here must not get dropped until the operation is
    // complete.
    let parent = parent.window().window_handle();
    let display = parent.display_handle();
    let display = display.as_ref().map(|v| v.as_ref()).ok();

    // Get parent handle.
    let parent = parent.window_handle();
    let parent = parent.as_ref().map(|v| v.as_ref()).unwrap();
    let parent = WindowIdentifier::from_raw_handle(parent, display).await;

    f(parent).await
}
