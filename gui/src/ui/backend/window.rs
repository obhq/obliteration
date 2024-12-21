use crate::rt::RuntimeWindow;
use i_slint_core::window::WindowAdapterInternal;
use i_slint_core::InternalToken;
use i_slint_renderer_skia::SkiaRenderer;
use slint::platform::{Renderer, WindowAdapter, WindowEvent, WindowProperties};
use slint::{LogicalPosition, LogicalSize, PhysicalSize, PlatformError, WindowSize};
use std::any::Any;
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;
use winit::event::{DeviceId, InnerSizeWriter};
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
    fn on_resized(
        &self,
        new: winit::dpi::PhysicalSize<u32>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let size = PhysicalSize::new(new.width, new.height);
        let size = LogicalSize::from_physical(size, self.winit.scale_factor() as f32);

        self.slint.dispatch_event(WindowEvent::Resized { size });

        Ok(())
    }

    fn on_focused(&self, gained: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.slint
            .dispatch_event(WindowEvent::WindowActiveChanged(gained));

        Ok(())
    }

    fn on_cursor_moved(
        &self,
        _: DeviceId,
        pos: winit::dpi::PhysicalPosition<f64>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let pos = pos.to_logical(self.winit.scale_factor());
        let position = LogicalPosition::new(pos.x, pos.y);

        self.slint
            .dispatch_event(WindowEvent::PointerMoved { position });

        Ok(())
    }

    fn on_scale_factor_changed(
        &self,
        new: f64,
        _: InnerSizeWriter,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let scale_factor = new as f32;

        self.slint
            .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });

        Ok(())
    }

    fn on_redraw_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
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

    fn set_size(&self, size: WindowSize) {
        todo!()
    }

    fn size(&self) -> PhysicalSize {
        let s = self.winit.inner_size();

        PhysicalSize::new(s.width, s.height)
    }

    fn request_redraw(&self) {
        self.winit.request_redraw();
    }

    fn renderer(&self) -> &dyn Renderer {
        &self.renderer
    }

    fn update_window_properties(&self, properties: WindowProperties) {
        // Set window size.
        let size = properties.layout_constraints();
        let scale = self.winit.scale_factor() as f32;
        let map = move |v: LogicalSize| {
            let v = v.to_physical(scale);
            let v = winit::dpi::PhysicalSize::new(v.width, v.height);

            winit::dpi::Size::from(v)
        };

        self.winit.set_min_inner_size(size.min.map(&map));
        self.winit.set_max_inner_size(size.max.map(&map));

        let _ = self.winit.request_inner_size(map(size.preferred));
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
