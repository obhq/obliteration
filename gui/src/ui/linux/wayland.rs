use super::PlatformError;
use crate::ui::DesktopWindow;
use crate::ui::backend::Wayland;
use wayland_backend::sys::client::ObjectId;
use wayland_client::Proxy;
use wayland_protocols::xdg::dialog::v1::client::xdg_dialog_v1::XdgDialogV1;
use wayland_protocols::xdg::shell::client::xdg_toplevel::XdgToplevel;

/// # Safety
/// `parent` must outlive `target`.
pub unsafe fn set_modal(
    wayland: &Wayland,
    target: &impl DesktopWindow,
    parent: &impl DesktopWindow,
) -> Result<XdgDialogV1, PlatformError> {
    // Get xdg_toplevel for parent.
    let mut queue = wayland.queue().borrow_mut();
    let mut state = wayland.state().borrow_mut();
    let qh = queue.handle();
    let parent = unsafe { get_xdg_toplevel(wayland, parent) };

    // Get xdg_dialog_v1.
    let target = unsafe { get_xdg_toplevel(wayland, target) };
    let dialog = state.xdg_dialog().get_xdg_dialog(&target, &qh, ());

    queue
        .roundtrip(&mut state)
        .map_err(PlatformError::CreateXdgDialogV1)?;

    // Set modal.
    target.set_parent(Some(&parent));
    dialog.set_modal();

    queue
        .roundtrip(&mut state)
        .map_err(PlatformError::SetModal)?;

    Ok(dialog)
}

/// # Safety
/// `win` must outlive the returned [`XdgToplevel`].
unsafe fn get_xdg_toplevel(wayland: &Wayland, win: &impl DesktopWindow) -> XdgToplevel {
    let obj = win.xdg_toplevel();
    let obj = unsafe { ObjectId::from_ptr(XdgToplevel::interface(), obj.cast()).unwrap() };

    XdgToplevel::from_id(wayland.connection(), obj).unwrap()
}
