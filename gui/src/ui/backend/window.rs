use i_slint_core::window::WindowAdapterInternal;
use i_slint_core::InternalToken;
use i_slint_renderer_skia::SkiaRenderer;
use slint::platform::{Renderer, WindowAdapter};
use slint::{PhysicalSize, PlatformError};
use std::any::Any;
use std::rc::Rc;
use winit::window::WindowId;

/// Implementation of [`WindowAdapter`].
pub struct Window {
    winit: Rc<winit::window::Window>,
    slint: slint::Window,
    renderer: SkiaRenderer,
}

impl Window {
    pub fn new(
        winit: Rc<winit::window::Window>,
        slint: slint::Window,
        renderer: SkiaRenderer,
    ) -> Self {
        Self {
            winit,
            slint,
            renderer,
        }
    }

    pub fn id(&self) -> WindowId {
        self.winit.id()
    }
}

impl WindowAdapter for Window {
    fn window(&self) -> &slint::Window {
        &self.slint
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        todo!()
    }

    fn size(&self) -> PhysicalSize {
        let s = self.winit.inner_size();

        PhysicalSize::new(s.width, s.height)
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn internal(&self, _: InternalToken) -> Option<&dyn WindowAdapterInternal> {
        Some(self)
    }
}

impl WindowAdapterInternal for Window {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
