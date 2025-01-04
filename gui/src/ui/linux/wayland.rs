use super::PlatformError;
use crate::ui::Wayland;
use raw_window_handle::HasWindowHandle;

pub fn set_modal(
    _: &Wayland,
    _: impl HasWindowHandle,
    _: impl HasWindowHandle,
) -> Result<(), PlatformError> {
    // TODO: We need xdg_toplevel from the target window to use xdg_wm_dialog_v1::get_xdg_dialog.
    // AFAIK the only way to get it is using xdg_surface::get_toplevel. The problem is
    // xdg_wm_base::get_xdg_surface that return xdg_surface can be called only once per wl_surface
    // and this call already done by winit. So we need winit to expose either xdg_surface or
    // xdg_toplevel in order for us to implement this.
    Ok(())
}
