use i_slint_core::InternalToken;
use i_slint_core::window::WindowAdapterInternal;
use i_slint_renderer_skia::SkiaRenderer;
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle, WindowHandle,
};
use slint::platform::{
    Key, PointerEventButton, Renderer, WindowAdapter, WindowEvent, WindowProperties,
};
use slint::{LogicalPosition, LogicalSize, PhysicalSize, PlatformError, SharedString};
use std::any::Any;
use std::cell::Cell;
use std::error::Error;
use std::rc::Rc;
use wae::{Signal, WindowHandler, WinitWindow};
use winit::event::{DeviceId, ElementState, InnerSizeWriter, MouseButton};
use winit::window::WindowId;

/// Implementation of [`slint::platform::WindowAdapter`].
pub struct SlintWindow {
    winit: Rc<winit::window::Window>,
    slint: slint::Window,
    renderer: SkiaRenderer,
    visible: Cell<Option<bool>>, // Wayland does not support this so we need to emulate it.
    hidden: Signal<()>,
    pointer: Cell<LogicalPosition>,
    title: Cell<SharedString>,
    minimum_size: Cell<Option<winit::dpi::PhysicalSize<u32>>>,
    maximum_size: Cell<Option<winit::dpi::PhysicalSize<u32>>>,
    preferred_size: Cell<Option<winit::dpi::PhysicalSize<u32>>>,
}

impl SlintWindow {
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
            hidden: Signal::default(),
            pointer: Cell::default(),
            title: Cell::default(),
            minimum_size: Cell::default(),
            maximum_size: Cell::default(),
            preferred_size: Cell::default(),
        }
    }

    pub fn from_adapter(adapter: &dyn WindowAdapter) -> &Self {
        adapter
            .internal(InternalToken)
            .unwrap()
            .as_any()
            .downcast_ref::<Self>()
            .unwrap()
    }

    pub fn winit(&self) -> &winit::window::Window {
        &self.winit
    }

    pub fn hidden(&self) -> &Signal<()> {
        &self.hidden
    }

    #[cfg(target_os = "macos")]
    fn build_menu(
        mtm: objc2::MainThreadMarker,
        bar: &vtable::VBox<i_slint_core::menus::MenuVTable>,
        menu: &objc2_app_kit::NSMenu,
        items: slint::SharedVector<i_slint_core::items::MenuEntry>,
    ) {
        use objc2::MainThreadOnly;
        use objc2_app_kit::{NSMenu, NSMenuItem};
        use objc2_foundation::{NSString, ns_string};
        use slint::SharedVector;

        for item in items {
            // Get sub-menu.
            let mut items = SharedVector::default();

            bar.sub_menu(Some(&item), &mut items);

            // Create NSMenuItem.
            let title = NSString::from_str(&item.title);
            let item = NSMenuItem::alloc(mtm);
            let item = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(item, &title, None, ns_string!(""))
            };

            // Create sub-menu.
            if !items.is_empty() {
                let menu = NSMenu::alloc(mtm);
                let menu = unsafe { NSMenu::initWithTitle(menu, &title) };

                Self::build_menu(mtm, bar, &menu, items);

                item.setSubmenu(Some(&menu));
            }

            menu.addItem(&item);
        }
    }
}

impl WinitWindow for SlintWindow {
    fn id(&self) -> WindowId {
        self.winit.id()
    }
}

impl WindowHandler for SlintWindow {
    fn on_resized(
        &self,
        new: winit::dpi::PhysicalSize<u32>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let size = PhysicalSize::new(new.width, new.height);
        let size = LogicalSize::from_physical(size, self.winit.scale_factor() as f32);

        self.slint.dispatch_event(WindowEvent::Resized { size });

        Ok(())
    }

    fn on_close_requested(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.slint.dispatch_event(WindowEvent::CloseRequested);
        Ok(())
    }

    fn on_focused(&self, gained: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.slint
            .dispatch_event(WindowEvent::WindowActiveChanged(gained));

        Ok(())
    }

    fn on_keyboard_input(
        &self,
        _: DeviceId,
        event: winit::event::KeyEvent,
        _: bool,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use winit::keyboard::{KeyCode, PhysicalKey};

        // Get text.
        let text = match event.physical_key {
            PhysicalKey::Code(KeyCode::Backspace) => Key::Backspace.into(),
            PhysicalKey::Code(KeyCode::Tab) => Key::Tab.into(),
            PhysicalKey::Code(KeyCode::Enter) => Key::Return.into(),
            PhysicalKey::Code(KeyCode::Escape) => Key::Escape.into(),
            PhysicalKey::Code(KeyCode::Delete) => Key::Delete.into(),
            PhysicalKey::Code(KeyCode::ShiftLeft) => Key::Shift.into(),
            PhysicalKey::Code(KeyCode::ShiftRight) => Key::ShiftR.into(),
            PhysicalKey::Code(KeyCode::ControlLeft) => Key::Control.into(),
            PhysicalKey::Code(KeyCode::ControlRight) => Key::ControlR.into(),
            PhysicalKey::Code(KeyCode::AltLeft) => Key::Alt.into(),
            PhysicalKey::Code(KeyCode::AltRight) => Key::AltGr.into(),
            PhysicalKey::Code(KeyCode::CapsLock) => Key::CapsLock.into(),
            PhysicalKey::Code(KeyCode::SuperLeft) => Key::Meta.into(),
            PhysicalKey::Code(KeyCode::SuperRight) => Key::MetaR.into(),
            PhysicalKey::Code(KeyCode::Space) => Key::Space.into(),
            PhysicalKey::Code(KeyCode::ArrowUp) => Key::UpArrow.into(),
            PhysicalKey::Code(KeyCode::ArrowDown) => Key::DownArrow.into(),
            PhysicalKey::Code(KeyCode::ArrowLeft) => Key::LeftArrow.into(),
            PhysicalKey::Code(KeyCode::ArrowRight) => Key::RightArrow.into(),
            PhysicalKey::Code(KeyCode::F1) => Key::F1.into(),
            PhysicalKey::Code(KeyCode::F2) => Key::F2.into(),
            PhysicalKey::Code(KeyCode::F3) => Key::F3.into(),
            PhysicalKey::Code(KeyCode::F4) => Key::F4.into(),
            PhysicalKey::Code(KeyCode::F5) => Key::F5.into(),
            PhysicalKey::Code(KeyCode::F6) => Key::F6.into(),
            PhysicalKey::Code(KeyCode::F7) => Key::F7.into(),
            PhysicalKey::Code(KeyCode::F8) => Key::F8.into(),
            PhysicalKey::Code(KeyCode::F9) => Key::F9.into(),
            PhysicalKey::Code(KeyCode::F10) => Key::F10.into(),
            PhysicalKey::Code(KeyCode::F11) => Key::F11.into(),
            PhysicalKey::Code(KeyCode::F12) => Key::F12.into(),
            PhysicalKey::Code(KeyCode::F13) => Key::F13.into(),
            PhysicalKey::Code(KeyCode::F14) => Key::F14.into(),
            PhysicalKey::Code(KeyCode::F15) => Key::F15.into(),
            PhysicalKey::Code(KeyCode::F16) => Key::F16.into(),
            PhysicalKey::Code(KeyCode::F17) => Key::F17.into(),
            PhysicalKey::Code(KeyCode::F18) => Key::F18.into(),
            PhysicalKey::Code(KeyCode::F19) => Key::F19.into(),
            PhysicalKey::Code(KeyCode::F20) => Key::F20.into(),
            PhysicalKey::Code(KeyCode::F21) => Key::F21.into(),
            PhysicalKey::Code(KeyCode::F22) => Key::F22.into(),
            PhysicalKey::Code(KeyCode::F23) => Key::F23.into(),
            PhysicalKey::Code(KeyCode::F24) => Key::F24.into(),
            PhysicalKey::Code(KeyCode::Insert) => Key::Insert.into(),
            PhysicalKey::Code(KeyCode::Home) => Key::Home.into(),
            PhysicalKey::Code(KeyCode::End) => Key::End.into(),
            PhysicalKey::Code(KeyCode::PageUp) => Key::PageUp.into(),
            PhysicalKey::Code(KeyCode::PageDown) => Key::PageDown.into(),
            PhysicalKey::Code(KeyCode::ScrollLock) => Key::ScrollLock.into(),
            PhysicalKey::Code(KeyCode::Pause) => Key::Pause.into(),
            PhysicalKey::Code(KeyCode::PrintScreen) => Key::SysReq.into(),
            PhysicalKey::Code(KeyCode::ContextMenu) => Key::Menu.into(),
            _ => match event.text {
                Some(v) => v.as_str().into(),
                None => return Ok(()),
            },
        };

        // Map to Slint event.
        let ev = match event.state {
            ElementState::Pressed => {
                if event.repeat {
                    WindowEvent::KeyPressRepeated { text }
                } else {
                    WindowEvent::KeyPressed { text }
                }
            }
            ElementState::Released => WindowEvent::KeyReleased { text },
        };

        self.slint.dispatch_event(ev);

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
        self.pointer.set(position);

        Ok(())
    }

    fn on_cursor_left(&self, _: DeviceId) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.slint.dispatch_event(WindowEvent::PointerExited);
        Ok(())
    }

    fn on_mouse_input(
        &self,
        _: DeviceId,
        st: ElementState,
        btn: MouseButton,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Map button.
        let button = match btn {
            MouseButton::Left => PointerEventButton::Left,
            MouseButton::Right => PointerEventButton::Right,
            MouseButton::Middle => PointerEventButton::Middle,
            MouseButton::Back => PointerEventButton::Back,
            MouseButton::Forward => PointerEventButton::Forward,
            MouseButton::Other(_) => PointerEventButton::Other,
        };

        // Dispatch to Slint.
        let position = self.pointer.get();
        let ev = match st {
            ElementState::Pressed => WindowEvent::PointerPressed { position, button },
            ElementState::Released => WindowEvent::PointerReleased { position, button },
        };

        self.slint.dispatch_event(ev);

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

            if self.slint.has_active_animations() {
                self.winit.request_redraw();
            }
        }

        Ok(())
    }
}

impl WindowAdapter for SlintWindow {
    fn window(&self) -> &slint::Window {
        &self.slint
    }

    fn set_visible(&self, visible: bool) -> Result<(), PlatformError> {
        if visible {
            assert!(self.visible.get().is_none());

            self.winit.set_visible(true);

            let is_wayland = match self.winit.display_handle().unwrap().as_raw() {
                RawDisplayHandle::Wayland(_) => true,
                _ => false,
            };

            // Render initial frame on macOS. Without this the modal will show a blank window until
            // show animation is complete. On Wayland there are some problems when another window is
            // showing so we need to to disable it.
            // On X11, this fixes the scaling.
            if !is_wayland || cfg!(target_os = "macos") {
                let scale_factor = self.winit.scale_factor() as f32;
                let size = self.winit.inner_size();
                let size = PhysicalSize::new(size.width, size.height);
                let size = LogicalSize::from_physical(size, scale_factor);

                self.slint
                    .dispatch_event(WindowEvent::ScaleFactorChanged { scale_factor });
                self.slint.dispatch_event(WindowEvent::Resized { size });

                self.renderer.render()?;
            }

            self.visible.set(Some(true));
        } else if self.visible.get().is_some_and(|v| v) {
            self.winit.set_visible(false);
            self.visible.set(Some(false));
            self.hidden.set(()).unwrap();
        }

        Ok(())
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
        // Set window title.
        let title = properties.title();

        if self.title.replace(title.clone()) != title {
            self.winit.set_title(&title);
        }

        // Setup mapper.
        let scale = self.winit.scale_factor() as f32;
        let map = move |v: LogicalSize| {
            let v = v.to_physical(scale);

            winit::dpi::PhysicalSize::new(v.width, v.height)
        };

        // Set window size.
        let size = properties.layout_constraints();
        let min = size.min.map(&map);
        let max = size.max.map(&map);
        let pre = map(size.preferred);

        if self.minimum_size.replace(min) != min {
            self.winit.set_min_inner_size(min);
        }

        if self.maximum_size.replace(max) != max {
            self.winit.set_max_inner_size(max);
        }

        // Winit on Wayland will panic if either width or height is zero.
        // TODO: Not sure why Slint also update the preferred size when window size is changed.
        if self.preferred_size.replace(Some(pre)).is_none() && pre.width != 0 && pre.height != 0 {
            let _ = self.winit.request_inner_size(pre);

            if matches!((min, max), (Some(min), Some(max)) if min == max && pre == max) {
                self.winit.set_resizable(false);
            }
        }
    }

    fn internal(&self, _: InternalToken) -> Option<&dyn WindowAdapterInternal> {
        Some(self)
    }

    fn window_handle_06(&self) -> Result<WindowHandle<'_>, HandleError> {
        self.winit.window_handle()
    }

    fn display_handle_06(&self) -> Result<DisplayHandle<'_>, HandleError> {
        self.winit.display_handle()
    }
}

impl WindowAdapterInternal for SlintWindow {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn supports_native_menu_bar(&self) -> bool {
        cfg!(target_os = "macos")
    }

    #[cfg(target_os = "macos")]
    fn setup_menubar(&self, bar: vtable::VBox<i_slint_core::menus::MenuVTable>) {
        use objc2::MainThreadMarker;
        use objc2_app_kit::NSApp;
        use slint::SharedVector;

        // Get menus on the menu bar.
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApp(mtm);
        let menu = unsafe { app.mainMenu().unwrap() };
        let mut items = SharedVector::default();

        bar.sub_menu(None, &mut items);

        Self::build_menu(mtm, &bar, &menu, items);
    }
}
