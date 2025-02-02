use super::Metal;
use crate::ui::DesktopWindow;
use metal::foreign_types::ForeignType;
use metal::MetalLayer;
use objc2::ffi::YES;
use objc2::msg_send;
use objc2::runtime::NSObject;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::error::Error;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;
use wae::{Hook, WindowHandler, WinitWindow};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton, StartCause};
use winit::event_loop::ControlFlow;
use winit::window::{Window, WindowId};

/// Implementation of [`WindowHandler`] and [`Hook`] for Metal.
///
/// Fields in this struct must be dropped in a correct order.
pub struct MetalWindow {
    layer: MetalLayer,
    window: Window,
    engine: Arc<Metal>,
}

impl MetalWindow {
    pub fn new(
        engine: &Arc<Metal>,
        window: Window,
    ) -> Result<Rc<Self>, Box<dyn Error + Send + Sync>> {
        let layer = unsafe { engine.create_layer() };
        let view = match window.window_handle().unwrap().as_ref() {
            RawWindowHandle::AppKit(v) => v.ns_view.as_ptr() as *mut NSObject,
            _ => unreachable!(),
        };

        let _: () = unsafe { msg_send![view, setLayer:layer.as_ptr() as *mut c_void] };
        let _: () = unsafe { msg_send![view, setWantsLayer:YES] };

        Ok(Rc::new(Self {
            layer,
            window,
            engine: engine.clone(),
        }))
    }
}

impl WinitWindow for MetalWindow {
    fn id(&self) -> WindowId {
        self.window.id()
    }
}

impl DesktopWindow for MetalWindow {
    fn handle(&self) -> impl HasWindowHandle + '_ {
        &self.window
    }
}

impl WindowHandler for MetalWindow {
    fn on_resized(&self, new: PhysicalSize<u32>) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_close_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_focused(&self, gained: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_cursor_moved(
        &self,
        dev: DeviceId,
        pos: PhysicalPosition<f64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_cursor_left(&self, dev: DeviceId) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_mouse_input(
        &self,
        dev: DeviceId,
        st: ElementState,
        btn: MouseButton,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_scale_factor_changed(
        &self,
        new: f64,
        sw: InnerSizeWriter,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn on_redraw_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }
}

impl Hook for MetalWindow {
    fn new_events(&self, cause: &StartCause) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn pre_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn window_destroyed(&self, id: WindowId) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn post_window_event(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        todo!()
    }

    fn about_to_wait(&self) -> Result<ControlFlow, Box<dyn Error + Send + Sync>> {
        todo!()
    }
}
