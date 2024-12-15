use crate::rt::RuntimeWindow;
use i_slint_core::window::WindowAdapterInternal;
use i_slint_core::InternalToken;
use i_slint_renderer_skia::SkiaRenderer;
use slint::platform::{Renderer, WindowAdapter, WindowEvent};
use slint::{PhysicalSize, PlatformError};
use std::any::Any;
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;
use winit::window::WindowId;

/// Implementation of [`WindowAdapter`].
pub struct Window {
    winit: Rc<winit::window::Window>,
    slint: slint::Window,
    renderer: SkiaRenderer,
    visible: Cell<Option<bool>>, // Wayland does not support this so we need to emulate it.
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
            visible: Cell::new(None),
        }
    }

    pub fn id(&self) -> WindowId {
        self.winit.id()
    }
}

impl RuntimeWindow for Window {
    fn update_scale_factor(&self, v: f64) -> Result<(), Box<dyn Error>> {
        self.slint.dispatch_event(WindowEvent::ScaleFactorChanged {
            scale_factor: v as f32,
        });

        Ok(())
    }

    fn redraw(&self) -> Result<(), Box<dyn Error>> {
        // Wayland will show the window on the first render so we need to check visibility flag
        // here.
        if self.visible.get().is_some_and(|v| v) {
            self.renderer.render()?;
        }

        Ok(())
    }
}

impl WindowAdapter for Window {
    fn window(&self) -> &slint::Window {
        &self.slint
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        if visible {
            assert!(self.visible.get().is_none());

            self.winit.set_visible(true);
            self.visible.set(Some(true));
        } else {
            assert_eq!(self.visible.get(), Some(true));

            self.winit.set_visible(false);
            self.visible.set(Some(false));
        }

        Ok(())
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
