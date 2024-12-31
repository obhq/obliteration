use super::BackendError;
use std::cell::RefCell;
use std::future::Future;
use std::hint::unreachable_unchecked;
use std::pin::Pin;
use std::task::{ready, Context, Poll};
use wayland_backend::sys::client::Backend;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::xdg::dialog::v1::client::xdg_wm_dialog_v1::XdgWmDialogV1;
use wayland_protocols::xdg::shell::client::xdg_wm_base::XdgWmBase;

/// Contains global objects for Wayland.
pub struct Wayland {
    queue: RefCell<EventQueue<WaylandState>>,
    state: RefCell<WaylandState>,
    conn: Connection,
}

impl Wayland {
    /// # Safety
    /// `display` must outlive the returned [`Wayland`].
    pub unsafe fn new(display: rwh05::WaylandDisplayHandle) -> Result<Self, BackendError> {
        // Get wayland connection.
        let backend = Backend::from_foreign_display(display.display.cast());
        let conn = Connection::from_backend(backend);

        // Get global objects.
        let (globals, mut queue) = registry_queue_init::<WaylandState>(&conn)
            .map_err(BackendError::RetrieveWaylandGlobals)?;
        let qh = queue.handle();

        // Get xdg_wm_base.
        let v = XdgWmBase::interface().version;
        let xdg_base: XdgWmBase = globals
            .bind(&qh, v..=v, ())
            .map_err(BackendError::BindXdgWmBase)?;

        // Get xdg_wm_dialog_v1.
        let v = XdgWmDialogV1::interface().version;
        let xdg_dialog: XdgWmDialogV1 = match globals.bind(&qh, v..=v, ()) {
            Ok(v) => v,
            Err(e) => {
                xdg_base.destroy();
                return Err(BackendError::BindXdgWmDialogV1(e));
            }
        };

        // Dispatch initial requests.
        let mut state = WaylandState {
            xdg_dialog,
            xdg_base,
        };

        queue
            .roundtrip(&mut state)
            .map_err(BackendError::DispatchWayland)?;

        Ok(Self {
            queue: RefCell::new(queue),
            state: RefCell::new(state),
            conn,
        })
    }

    pub fn queue(&self) -> &RefCell<EventQueue<WaylandState>> {
        &self.queue
    }

    pub fn state(&self) -> &RefCell<WaylandState> {
        &self.state
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn run(&self) -> impl Future<Output = ()> + '_ {
        Run(self)
    }
}

/// Provides [`Dispatch`] implementation to handle Wayland events.
pub struct WaylandState {
    xdg_dialog: XdgWmDialogV1,
    xdg_base: XdgWmBase,
}

impl Drop for WaylandState {
    fn drop(&mut self) {
        self.xdg_dialog.destroy();
        self.xdg_base.destroy();
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for WaylandState {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgWmBase, ()> for WaylandState {
    fn event(
        _: &mut Self,
        proxy: &XdgWmBase,
        event: <XdgWmBase as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use wayland_protocols::xdg::shell::client::xdg_wm_base::Event;

        match event {
            Event::Ping { serial } => proxy.pong(serial),
            _ => (),
        }
    }
}

impl Dispatch<XdgWmDialogV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &XdgWmDialogV1,
        _: <XdgWmDialogV1 as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

/// Implementation of [`Future`] to dispatch pending events for our queue.
struct Run<'a>(&'a Wayland);

impl<'a> Future for Run<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut queue = self.0.queue.borrow_mut();
        let mut state = self.0.state.borrow_mut();

        ready!(queue.poll_dispatch_pending(cx, &mut state)).unwrap();

        // SAFETY: The Ok from from poll_dispatch_pending is Infallible, which mean it is impossible
        // to construct Ok.
        unsafe { unreachable_unchecked() };
    }
}
