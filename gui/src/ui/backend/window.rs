use i_slint_renderer_skia::SkiaRenderer;
use slint::platform::{Renderer, WindowAdapter};
use slint::PhysicalSize;
use std::rc::Rc;

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
}

impl WindowAdapter for Window {
    fn window(&self) -> &slint::Window {
        &self.slint
    }

    fn size(&self) -> PhysicalSize {
        let s = self.winit.inner_size();

        PhysicalSize::new(s.width, s.height)
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }
}
