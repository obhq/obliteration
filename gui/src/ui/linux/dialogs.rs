use super::PlatformError;
use crate::ui::{DesktopWindow, FileType, SlintBackend};
use ashpd::desktop::file_chooser::{FileFilter, SelectedFiles};
use ashpd::desktop::ResponseError;
use ashpd::WindowIdentifier;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use wayland_backend::sys::client::ObjectId;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::Proxy;
use wayland_protocols::xdg::foreign::zv2::client::zxdg_exported_v2::ZxdgExportedV2;

pub async fn open_file<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
    ty: FileType,
) -> Result<Option<PathBuf>, PlatformError> {
    // Build filter.
    let filter = match ty {
        FileType::Firmware => FileFilter::new("Firmware Dump").glob("*.obf"),
    };

    // Send the request.
    let parent = get_parent_id(parent);
    let req = SelectedFiles::open_file()
        .identifier(parent.id)
        .title(title.as_ref())
        .modal(true)
        .filter(filter)
        .send()
        .await;

    if let Some(v) = parent.surface {
        v.destroy();
    }

    // Get response.
    let resp = match req.unwrap().response() {
        Ok(v) => v,
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => return Ok(None),
        Err(_) => unimplemented!(),
    };

    // Get file path.
    Ok(Some(resp.uris().first().unwrap().to_file_path().unwrap()))
}

pub async fn open_dir<T: DesktopWindow>(
    parent: &T,
    title: impl AsRef<str>,
) -> Result<Option<PathBuf>, PlatformError> {
    // Send the request
    let parent = get_parent_id(parent);
    let req = SelectedFiles::open_file()
        .identifier(parent.id)
        .title(title.as_ref())
        .modal(true)
        .directory(true)
        .send()
        .await;

    if let Some(v) = parent.surface {
        v.destroy();
    }

    // Get response.
    let resp = match req.unwrap().response() {
        Ok(v) => v,
        Err(ashpd::Error::Response(ResponseError::Cancelled)) => return Ok(None),
        Err(_) => unimplemented!(),
    };

    // Get directory path.
    Ok(Some(resp.uris().first().unwrap().to_file_path().unwrap()))
}

fn get_parent_id<P>(parent: &P) -> Parent
where
    P: DesktopWindow,
{
    // Check window type.
    let parent = parent.handle();
    let parent = parent.window_handle().unwrap();
    let surface = match parent.as_ref() {
        RawWindowHandle::Xlib(v) => {
            return Parent {
                id: WindowIdentifier::from_xid(v.window),
                surface: None,
            }
        }
        RawWindowHandle::Xcb(v) => {
            return Parent {
                id: WindowIdentifier::from_xid(v.window.get().into()),
                surface: None,
            }
        }
        RawWindowHandle::Wayland(v) => v.surface.as_ptr(),
        RawWindowHandle::Drm(_) | RawWindowHandle::Gbm(_) => unimplemented!(),
        _ => unreachable!(),
    };

    // Get WlSurface.
    let backend = wae::global::<SlintBackend>().unwrap();
    let wayland = backend.wayland().unwrap();
    let surface = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.cast()).unwrap() };
    let surface = WlSurface::from_id(wayland.connection(), surface).unwrap();

    // Export surface.
    let mut queue = wayland.queue().borrow_mut();
    let mut state = wayland.state().borrow_mut();
    let qh = queue.handle();
    let handle = Arc::new(OnceLock::<String>::new());
    let surface = state
        .xdg_exporter()
        .export_toplevel(&surface, &qh, handle.clone());

    queue.roundtrip(&mut state).unwrap();

    // Construct WindowIdentifier. We need some hack here since we can't construct
    // WindowIdentifier::Wayland.
    let id = format!("wayland:{}", handle.get().unwrap());
    let id = WindowIdentifier::X11(id.parse().unwrap());

    Parent {
        id,
        surface: Some(surface),
    }
}

/// Encapsulates [`WindowIdentifier`] for parent window.
struct Parent {
    id: WindowIdentifier,
    surface: Option<ZxdgExportedV2>,
}
