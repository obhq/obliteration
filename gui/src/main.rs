#![windows_subsystem = "windows"]

use self::data::{DataError, DataMgr};
use self::gdb::{GdbDispatcher, GdbError, GdbHandler, GdbSession};
use self::graphics::{GraphicsBuilder, GraphicsError};
use self::log::LogWriter;
use self::profile::{DisplayResolution, Profile};
use self::settings::{Settings, SettingsError};
use self::setup::{SetupError, run_setup};
use self::ui::{
    AboutWindow, App, CpuList, DesktopExt, DeviceModel, EditEnvironment, MainWindow, NewProfile,
    ProductList, ProfileModel, ResolutionModel, RuntimeExt, SettingsWindow, WaitForDebugger, error,
    spawn_handler,
};
use self::vmm::{CpuError, Vmm, VmmError, VmmEvent};
use async_net::{TcpListener, TcpStream};
use clap::{Parser, ValueEnum};
use erdp::ErrorDisplay;
use futures::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, FutureExt, TryStreamExt, select_biased,
};
use hv::{DebugEvent, Hypervisor};
use slint::{ComponentHandle, SharedString, ToSharedString};
use std::cell::{Cell, RefMut};
use std::net::SocketAddr;
use std::num::NonZero;
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;
use thiserror::Error;
use winit::dpi::PhysicalSize;
use winit::window::Window;

mod data;
mod gdb;
mod graphics;
mod hw;
mod log;
mod panic;
mod profile;
mod settings;
mod setup;
mod ui;
mod util;
mod vfs;
mod vmm;

fn main() -> ExitCode {
    // Check program mode.
    let args = ProgramArgs::parse();

    match &args.mode {
        Some(ProgramMode::PanicHandler) => return self::panic::run_handler(),
        None => {}
    }

    // Run.
    self::ui::run::<MainProgram>(args)
}

/// Implementation of [`App`] for main program.
struct MainProgram {
    args: ProgramArgs,
    exe: PathBuf,
}

impl MainProgram {
    async fn run_launcher<G: GraphicsBuilder>(
        data: &Arc<DataMgr>,
        settings: &Rc<Settings>,
        graphics: G,
        profiles: Vec<Profile>,
    ) -> Result<Option<Launch<G>>, ProgramError> {
        // Create window and register callback handlers.
        let win = MainWindow::new().map_err(ProgramError::CreateMainWindow)?;
        let exit = Rc::new(Cell::new(None));
        let graphics = Rc::new(graphics);
        let devices = Rc::new(DeviceModel::new(graphics.clone()));
        let resolutions = Rc::new(ResolutionModel::default());
        let cpus = Rc::new(CpuList::default());
        let products = Rc::new(ProductList::default());
        let profiles = Rc::new(ProfileModel::new(
            &win,
            profiles,
            devices.clone(),
            resolutions.clone(),
            cpus.clone(),
            products.clone(),
        ));

        win.on_settings({
            let win = win.as_weak();
            let data = data.clone();
            let settings = settings.clone();

            move || spawn_handler(&win, |w| Self::settings(w, data.clone(), settings.clone()))
        });

        win.on_new_profile({
            let win = win.as_weak();
            let data = data.clone();
            let profiles = profiles.clone();

            move || {
                spawn_handler(&win, |w| {
                    Self::new_profile(w, data.clone(), profiles.clone())
                })
            }
        });

        win.on_report_issue({
            let win = win.as_weak();

            move || spawn_handler(&win, |_| Self::report_issue())
        });

        win.on_about({
            let win = win.as_weak();

            move || spawn_handler(&win, Self::about)
        });

        win.on_profile_selected({
            let win = win.as_weak();
            let profiles = profiles.clone();

            move || {
                // TODO: Check if previous profile has unsaved data before switch the profile.
                let win = win.unwrap();
                let row: usize = win.get_selected_profile().try_into().unwrap();

                profiles.select(row);
            }
        });

        win.on_save_profile({
            let data = data.clone();
            let win = win.as_weak();
            let profiles = profiles.clone();

            move || spawn_handler(&win, |_| Self::save_profile(data.clone(), profiles.clone()))
        });

        win.on_start_vmm({
            let win = win.as_weak();
            let profiles = profiles.clone();
            let exit = exit.clone();
            let ty = ExitAction::Run;

            move || {
                spawn_handler(&win, |w| {
                    Self::start_vmm(w, profiles.clone(), exit.clone(), ty)
                })
            }
        });

        win.on_start_debug({
            let win = win.as_weak();
            let profiles = profiles.clone();
            let exit = exit.clone();
            let ty = ExitAction::Debug;

            move || {
                spawn_handler(&win, |w| {
                    Self::start_vmm(w, profiles.clone(), exit.clone(), ty)
                })
            }
        });

        win.on_new_environment({
            let win = win.as_weak();
            let profiles = profiles.clone();

            move || spawn_handler(&win, |w| Self::new_environment(w, profiles.clone()))
        });

        win.on_edit_environment({
            let win = win.as_weak();
            let profiles = profiles.clone();

            move |row| spawn_handler(&win, |w| Self::edit_environment(w, profiles.clone(), row))
        });

        win.on_delete_environment({
            let win = win.as_weak();
            let profiles = profiles.clone();

            move |row| spawn_handler(&win, |_| Self::delete_environment(profiles.clone(), row))
        });

        // Set window properties.
        win.set_devices(devices.into());
        win.set_resolutions(resolutions.into());
        win.set_cpu_models(cpus.into());
        win.set_idps_products(products.into());
        win.set_profiles(profiles.clone().into());

        // Load selected profile.
        let row: usize = win.get_selected_profile().try_into().unwrap();

        profiles.select(row);

        // Run the window.
        win.show().map_err(ProgramError::ShowMainWindow)?;
        win.set_center().map_err(ProgramError::CenterMainWindow)?;
        win.wait().await;

        // Extract window states.
        let profile = win.get_selected_profile();

        drop(win);

        // Check how we exit.
        let exit = match Rc::into_inner(exit).unwrap().into_inner() {
            Some(v) => v,
            None => return Ok(None),
        };

        // Get selected profile.
        let mut profiles = Rc::into_inner(profiles).unwrap().into_inner();
        let profile = profiles.remove(profile.try_into().unwrap());

        Ok(Some(Launch {
            graphics: Rc::into_inner(graphics).unwrap(),
            profile,
            exit,
        }))
    }

    async fn settings(
        main: MainWindow,
        data: Arc<DataMgr>,
        settings: Rc<Settings>,
    ) -> Result<(), SharedString> {
        // Setup window.
        let win = SettingsWindow::new()
            .map_err(|e| slint::format!("Failed to create settings window: {}.", e.display()))?;

        win.on_cancel_clicked({
            let win = win.as_weak();

            move || win.unwrap().hide().unwrap()
        });

        win.on_ok_clicked({
            let win = win.as_weak();
            let data = data.clone();
            let settings = settings.clone();

            move || {
                spawn_handler(&win, |w| {
                    Self::save_settings(w, data.clone(), settings.clone())
                })
            }
        });

        win.set_graphics_debug_layer_name(if cfg!(target_os = "macos") {
            "MTL_DEBUG_LAYER".into()
        } else {
            "VK_LAYER_KHRONOS_validation".into()
        });

        win.set_graphics_debug_layer_checked(settings.graphics_debug_layer());

        // Run the window.
        win.show()
            .map_err(|e| slint::format!("Failed to show settings window: {}.", e.display()))?;
        win.set_modal(&main)
            .map_err(|e| {
                slint::format!(
                    "Failed to enable modal on settings window: {}.",
                    e.display()
                )
            })?
            .wait()
            .await;

        Ok(())
    }

    async fn save_settings(
        win: SettingsWindow,
        data: Arc<DataMgr>,
        settings: Rc<Settings>,
    ) -> Result<(), SharedString> {
        // Load values from window.
        settings.set_graphics_debug_layer(win.get_graphics_debug_layer_checked());

        // Save.
        let path = data.settings();

        settings.save(path).map_err(|e| {
            slint::format!(
                "Failed to save settings to {}: {}.",
                path.display(),
                e.display()
            )
        })?;

        // Close the window.
        win.hide()
            .map_err(|e| slint::format!("Failed to hide settings window: {}.", e.display()))?;

        Ok(())
    }

    async fn new_profile<G: GraphicsBuilder>(
        main: MainWindow,
        data: Arc<DataMgr>,
        profiles: Rc<ProfileModel<G>>,
    ) -> Result<(), SharedString> {
        // Setup window.
        let win = NewProfile::new()
            .map_err(|e| slint::format!("Failed to create window: {}.", e.display()))?;
        let index = Rc::new(Cell::new(None));

        win.on_cancel_clicked({
            let win = win.as_weak();

            move || win.unwrap().hide().unwrap()
        });

        win.on_ok_clicked({
            let win = win.as_weak();
            let data = data.clone();
            let profiles = profiles.clone();
            let index = index.clone();

            move || {
                spawn_handler(&win, |w| {
                    Self::create_profile(w, data.clone(), profiles.clone(), index.clone())
                })
            }
        });

        // Run the window.
        win.show()
            .map_err(|e| slint::format!("Failed to show window: {}.", e.display()))?;
        win.set_modal(&main)
            .map_err(|e| slint::format!("Failed to enable modal on window: {}.", e.display()))?
            .wait()
            .await;

        if let Some(i) = index.get() {
            main.set_selected_profile(i);
            main.invoke_profile_selected();
        }

        Ok(())
    }

    async fn create_profile<G>(
        win: NewProfile,
        data: Arc<DataMgr>,
        profiles: Rc<ProfileModel<G>>,
        index: Rc<Cell<Option<i32>>>,
    ) -> Result<(), SharedString> {
        // Get name.
        let name = win.get_name();

        if name.is_empty() {
            return Err("Name cannot be empty.".into());
        }

        // Create profile.
        let pf = Profile::new(name);
        let path = data.profiles().data(pf.id());

        std::fs::create_dir(&path)
            .map_err(|e| slint::format!("Failed to create {}: {}.", path.display(), e.display()))?;

        pf.save(&path).map_err(|e| {
            slint::format!(
                "Failed to save profile to {}: {}.",
                path.display(),
                e.display()
            )
        })?;

        win.hide()
            .map_err(|e| slint::format!("Failed to hide the window: {}.", e.display()))?;

        index.set(Some(profiles.push(pf)));

        Ok(())
    }

    async fn report_issue() -> Result<(), SharedString> {
        let url = "https://github.com/obhq/obliteration/issues/new";

        open::that_detached(url)
            .map_err(|e| slint::format!("Failed to open {}: {}.", url, e.display()))
    }

    async fn about(main: MainWindow) -> Result<(), SharedString> {
        // Setup window.
        let win = AboutWindow::new()
            .map_err(|e| slint::format!("Failed to create window: {}.", e.display()))?;

        win.on_close_clicked({
            let win = win.as_weak();

            move || win.unwrap().hide().unwrap()
        });

        // Run the window.
        win.show()
            .map_err(|e| slint::format!("Failed to show window: {}.", e.display()))?;
        win.set_modal(&main)
            .map_err(|e| slint::format!("Failed to enable modal on window: {}.", e.display()))?
            .wait()
            .await;

        Ok(())
    }

    async fn save_profile<G: GraphicsBuilder>(
        data: Arc<DataMgr>,
        profiles: Rc<ProfileModel<G>>,
    ) -> Result<(), SharedString> {
        let pf = Self::update_profile(&profiles)?;
        let loc = data.profiles().data(pf.id());

        pf.save(loc)
            .map_err(|e| slint::format!("Failed to save profile: {}.", e.display()))
    }

    async fn start_vmm<G: GraphicsBuilder>(
        win: MainWindow,
        profiles: Rc<ProfileModel<G>>,
        exit: Rc<Cell<Option<ExitAction>>>,
        ty: ExitAction,
    ) -> Result<(), SharedString> {
        Self::update_profile(&profiles)?;

        win.hide()
            .map_err(|e| slint::format!("Failed to hide window: {}.", e.display()))?;
        exit.set(Some(ty));

        Ok(())
    }

    fn update_profile<'a, G: GraphicsBuilder>(
        profiles: &'a ProfileModel<G>,
    ) -> Result<RefMut<'a, Profile>, SharedString> {
        let pro = profiles
            .update()
            .map_err(|e| slint::format!("Failed to update profile: {}.", e.display()))?;

        Ok(pro)
    }

    async fn new_environment<G: 'static>(
        main: MainWindow,
        profiles: Rc<ProfileModel<G>>,
    ) -> Result<(), SharedString> {
        // Setup window.
        let win = EditEnvironment::new()
            .map_err(|e| slint::format!("Failed to create window: {}.", e.display()))?;

        win.on_cancel_clicked({
            let win = win.as_weak();

            move || win.unwrap().hide().unwrap()
        });

        win.on_ok_clicked({
            let win = win.as_weak();

            move || spawn_handler(&win, |w| Self::create_environment(w, profiles.clone()))
        });

        win.set_dialog_title("New Variable".into());

        // Run the window.
        win.show()
            .map_err(|e| slint::format!("Failed to show window: {}.", e.display()))?;
        win.set_modal(&main)
            .map_err(|e| slint::format!("Failed to enable modal on window: {}.", e.display()))?
            .wait()
            .await;

        Ok(())
    }

    async fn create_environment<G>(
        win: EditEnvironment,
        profiles: Rc<ProfileModel<G>>,
    ) -> Result<(), SharedString> {
        let (name, value) = Self::validate_environment(&win)?;

        profiles.environments().push(name, value);
        win.hide().unwrap();

        Ok(())
    }

    async fn edit_environment<G: 'static>(
        main: MainWindow,
        profiles: Rc<ProfileModel<G>>,
        row: i32,
    ) -> Result<(), SharedString> {
        // Setup window.
        let row = row.try_into().unwrap();
        let (name, value) = profiles.get_env(row);
        let win = EditEnvironment::new().unwrap();

        win.on_cancel_clicked({
            let win = win.as_weak();

            move || win.unwrap().hide().unwrap()
        });

        win.on_ok_clicked({
            let win = win.as_weak();

            move || {
                spawn_handler(&win, |w| {
                    Self::apply_edited_environment(w, profiles.clone(), row)
                })
            }
        });

        win.set_dialog_title("Edit Variable".into());
        win.set_name(name);
        win.set_value(value);

        // Run the window.
        win.show().unwrap();
        win.set_modal(&main).unwrap().wait().await;

        Ok(())
    }

    async fn apply_edited_environment<G>(
        win: EditEnvironment,
        profiles: Rc<ProfileModel<G>>,
        row: usize,
    ) -> Result<(), SharedString> {
        let (name, value) = Self::validate_environment(&win)?;

        profiles.environments().set(row, name, value);
        win.hide().unwrap();

        Ok(())
    }

    fn validate_environment(
        win: &EditEnvironment,
    ) -> Result<(SharedString, SharedString), SharedString> {
        // Get name.
        let name = win.get_name();

        if name.is_empty() {
            return Err("Name cannot be empty.".into());
        }

        // Get value.
        let value = win.get_value();

        if value.is_empty() {
            return Err("Value cannot be empty.".into());
        }

        Ok((name, value))
    }

    async fn delete_environment<G>(
        profiles: Rc<ProfileModel<G>>,
        row: i32,
    ) -> Result<(), SharedString> {
        let row = row.try_into().unwrap();

        profiles.environments().remove(row);

        Ok(())
    }

    async fn wait_for_debugger(addr: SocketAddr) -> Result<Option<TcpStream>, ProgramError> {
        // Start server.
        let server = TcpListener::bind(addr)
            .await
            .map_err(|e| ProgramError::StartDebugServer(addr, e))?;
        let addr = server.local_addr().map_err(ProgramError::GetDebugAddr)?;

        // Tell the user that we are waiting for a debugger.
        let win = WaitForDebugger::new().map_err(ProgramError::CreateDebugWindow)?;

        win.set_address(addr.to_shared_string());
        win.show().map_err(ProgramError::ShowDebugWindow)?;

        // Wait for connection.
        let client = select_biased! {
            _ = win.wait().fuse() => return Ok(None),
            v = server.accept().fuse() => match v {
                Ok(v) => v.0,
                Err(e) => return Err(ProgramError::AcceptDebugger(e)),
            }
        };

        // Disable Nagle algorithm since it does not work well with GDB remote protocol.
        client
            .set_nodelay(true)
            .map_err(ProgramError::DisableDebuggerNagle)?;

        Ok(Some(client))
    }

    async fn dispatch_gdb<H: Hypervisor>(
        cx: &mut Context<H>,
        res: Result<usize, std::io::Error>,
        gdb: &mut GdbSession,
        buf: &[u8],
        con: &mut (dyn AsyncWrite + Unpin),
    ) -> Result<bool, ProgramError> {
        // Check status.
        let len = res.map_err(ProgramError::ReadDebuggerSocket)?;

        if len == 0 {
            return Ok(false);
        }

        // Dispatch the requests.
        let mut dis = gdb.dispatch_client(&buf[..len], cx);

        while let Some(res) = dis.pump().map_err(ProgramError::DispatchDebugger)? {
            let res = res.as_ref();

            if !res.is_empty() {
                con.write_all(res)
                    .await
                    .map_err(ProgramError::WriteDebuggerSocket)?;
            }
        }

        Ok(true)
    }
}

impl App for MainProgram {
    type Err = ProgramError;
    type Args = ProgramArgs;

    const NAME: &str = "main program";

    fn new(args: Self::Args) -> Result<Self, Self::Err> {
        // Spawn panic handler.
        let exe = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .map_err(ProgramError::GetExePath)?;

        self::panic::spawn_handler(&exe).map_err(ProgramError::SpawnPanicHandler)?;

        Ok(Self { args, exe })
    }

    async fn run(self) -> Result<(), Self::Err> {
        // Increase number of file descriptor to maximum allowed.
        #[cfg(unix)]
        unsafe {
            use libc::{RLIMIT_NOFILE, getrlimit, setrlimit};
            use std::io::Error;
            use std::mem::MaybeUninit;

            // Get current value.
            let mut val = MaybeUninit::uninit();

            if getrlimit(RLIMIT_NOFILE, val.as_mut_ptr()) < 0 {
                return Err(ProgramError::GetFdLimit(Error::last_os_error()));
            }

            // Check if we need to increase the limit.
            let mut val = val.assume_init();

            if val.rlim_cur < val.rlim_max {
                val.rlim_cur = val.rlim_max;

                if setrlimit(RLIMIT_NOFILE, &val) < 0 {
                    return Err(ProgramError::SetFdLimit(Error::last_os_error()));
                }
            }
        }

        // Run setup wizard. This will simply return the data manager if the user already has
        // required settings.
        let data = match run_setup().await.map_err(ProgramError::Setup)? {
            Some(v) => Arc::new(v),
            None => return Ok(()),
        };

        // Load application settings.
        let settings = match self.args.use_default_settings {
            true => Rc::new(Settings::default()),
            false => {
                let path = data.settings();
                let data =
                    Settings::load(path).map_err(|e| ProgramError::LoadSettings(path.into(), e))?;

                Rc::new(data)
            }
        };

        // Initialize graphics engine.
        let graphics = graphics::builder(&settings).map_err(ProgramError::InitGraphics)?;
        let kernel = self.args.kernel.as_ref().cloned().unwrap_or_else(|| {
            // Get kernel directory.
            let mut path = self.exe.parent().unwrap().to_owned();

            if cfg!(target_os = "windows") {
                path.push("share");
            } else {
                path.pop();

                if cfg!(target_os = "macos") {
                    path.push("Resources");
                } else {
                    path.push("share");
                }
            }

            // Append kernel.
            path.push("obkrnl");
            path
        });

        // Load profiles.
        let mut profiles = Vec::new();

        for l in data.profiles().list().map_err(ProgramError::ListProfile)? {
            let l = l.map_err(ProgramError::ListProfile)?;
            let p = Profile::load(&l).map_err(ProgramError::LoadProfile)?;

            profiles.push(p);
        }

        // Create default profile if user does not have any profiles.
        if profiles.is_empty() {
            // Create directory.
            let p = Profile::default();
            let l = data.profiles().data(p.id());

            if let Err(e) = std::fs::create_dir(&l) {
                return Err(ProgramError::CreateDirectory(l, e));
            }

            // Save.
            p.save(&l).map_err(ProgramError::SaveDefaultProfile)?;

            profiles.push(p);
        }

        // Get profile to use.
        let (graphics, profile, debug) = if let Some(v) = self.args.debug {
            // TODO: Select last used profile.
            (graphics, profiles.pop().unwrap(), Some(v))
        } else {
            let r = match Self::run_launcher(&data, &settings, graphics, profiles).await? {
                Some(v) => v,
                None => return Ok(()),
            };

            match r.exit {
                ExitAction::Run => (r.graphics, r.profile, None),
                ExitAction::Debug => {
                    let addr = r.profile.debug_addr;

                    (r.graphics, r.profile, Some(addr))
                }
            }
        };

        // Wait for debugger.
        let mut gdb_read: Box<dyn AsyncRead + Unpin>;
        let mut gdb_write: Box<dyn AsyncWrite + Unpin>;

        match debug {
            Some(addr) => {
                let (r, w) = match Self::wait_for_debugger(addr).await? {
                    Some(v) => v.split(),
                    None => return Ok(()),
                };

                gdb_read = Box::new(r);
                gdb_write = Box::new(w);
            }
            None => {
                gdb_read =
                    Box::new(futures::stream::pending::<Result<&[u8], _>>().into_async_read());
                gdb_write = Box::new(futures::io::sink());
            }
        }

        // Setup WindowAttributes for VMM screen.
        let attrs = Window::default_attributes()
            .with_inner_size(match profile.display_resolution {
                DisplayResolution::Hd => PhysicalSize::new(1280, 720),
                DisplayResolution::FullHd => PhysicalSize::new(1920, 1080),
                DisplayResolution::UltraHd => PhysicalSize::new(3840, 2160),
            })
            .with_resizable(false)
            .with_title("Obliteration");

        // Prepare to launch VMM.
        let logs = data.logs();
        let logs =
            LogWriter::new(logs).map_err(|e| ProgramError::CreateKernelLog(logs.into(), e))?;
        let shutdown = Arc::default();
        let graphics = graphics
            .build(&profile, attrs, &shutdown)
            .map_err(ProgramError::BuildGraphicsEngine)?;
        let mut gdb = GdbSession::default();
        let mut gdb_buf = [0; 1024];

        // Start VMM.
        let vmm = match Vmm::new(&profile, &kernel, &shutdown, debug.is_some()) {
            Ok(v) => v,
            Err(e) => return Err(ProgramError::StartVmm(kernel, e)),
        };

        // Dispatch events until shutdown.
        let mut cx = Context {
            vmm,
            logs,
            pending_bps: Vec::new(),
        };

        loop {
            // Wait for event.
            let r = select_biased! {
                v = gdb_read.read(&mut gdb_buf).fuse() => {
                    Self::dispatch_gdb(&mut cx, v, &mut gdb, &gdb_buf, &mut gdb_write).await?
                }
                v = cx.vmm.recv().fuse() => cx.dispatch_vmm(v.0, v.1).await?,
            };

            if !r {
                break;
            }
        }

        Ok(())
    }
}

/// Contains state for the main loop.
struct Context<H> {
    vmm: Vmm<H>,
    logs: LogWriter,
    pending_bps: Vec<(usize, Option<DebugEvent>)>,
}

impl<H: Hypervisor> Context<H> {
    async fn dispatch_vmm(
        &mut self,
        cpu: usize,
        ev: Option<VmmEvent>,
    ) -> Result<bool, ProgramError> {
        let ev = match ev {
            Some(v) => v,
            None => {
                let r = self.vmm.remove_cpu(cpu);

                if !r.map_err(ProgramError::CpuThread)? {
                    return Err(ProgramError::CpuPanic(cpu, self.logs.path().into()));
                } else if cpu == 0 {
                    return Ok(false);
                } else {
                    return Ok(true);
                }
            }
        };

        match ev {
            VmmEvent::Log(t, m) => self.logs.write(t, m),
            VmmEvent::Breakpoint(e) => self.pending_bps.push((cpu, e)),
            VmmEvent::Registers(_) => todo!(),
            VmmEvent::TranslatedAddress(_) => todo!(),
        }

        Ok(true)
    }
}

impl<H: Hypervisor> GdbHandler for Context<H> {
    fn active_thread(&mut self) -> impl IntoIterator<Item = NonZero<usize>> {
        self.vmm
            .active_cpus()
            .map(|v| v.checked_add(1).unwrap().try_into().unwrap())
    }
}

/// Program arguments parsed from command line.
#[derive(Parser)]
#[command(about = None)]
struct ProgramArgs {
    #[arg(long, value_enum, hide = true)]
    mode: Option<ProgramMode>,

    /// Immediate launch the VMM in debug mode.
    #[arg(long, value_name = "ADDR")]
    debug: Option<SocketAddr>,

    /// Use the kernel image at the specified path instead of the default one.
    #[arg(long, value_name = "PATH")]
    kernel: Option<PathBuf>,

    /// Ignore saved settings and use default values instead.
    #[arg(long)]
    use_default_settings: bool,
}

/// Mode of our program.
#[derive(Clone, ValueEnum)]
enum ProgramMode {
    PanicHandler,
}

/// Contains objects returned from [`MainProgram::run_launcher()`].
struct Launch<G> {
    graphics: G,
    profile: Profile,
    exit: ExitAction,
}

/// Action to be performed after the main window is closed.
#[derive(Clone, Copy)]
enum ExitAction {
    Run,
    Debug,
}

/// Represents an error when [`MainProgram`] fails.
#[derive(Debug, Error)]
enum ProgramError {
    #[error("couldn't get application executable path")]
    GetExePath(#[source] std::io::Error),

    #[error("couldn't spawn panic handler process")]
    SpawnPanicHandler(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("couldn't get file descriptor limit")]
    GetFdLimit(#[source] std::io::Error),

    #[cfg(unix)]
    #[error("couldn't increase file descriptor limit")]
    SetFdLimit(#[source] std::io::Error),

    #[error("couldn't run setup wizard")]
    Setup(#[source] SetupError),

    #[error("couldn't load settings from {0}")]
    LoadSettings(PathBuf, #[source] SettingsError),

    #[error("couldn't list available profiles")]
    ListProfile(#[source] DataError),

    #[error("couldn't load profile")]
    LoadProfile(#[source] self::profile::LoadError),

    #[error("couldn't create {0}")]
    CreateDirectory(PathBuf, #[source] std::io::Error),

    #[error("couldn't save default profile")]
    SaveDefaultProfile(#[source] self::profile::SaveError),

    #[error("couldn't start debug server on {0}")]
    StartDebugServer(SocketAddr, #[source] std::io::Error),

    #[error("couldn't get debug server address")]
    GetDebugAddr(#[source] std::io::Error),

    #[error("couldn't create debug server window")]
    CreateDebugWindow(#[source] slint::PlatformError),

    #[error("couldn't show debug server window")]
    ShowDebugWindow(#[source] slint::PlatformError),

    #[error("couldn't accept a connection from debugger")]
    AcceptDebugger(#[source] std::io::Error),

    #[error("couldn't disable Nagle algorithm on debugger connection")]
    DisableDebuggerNagle(#[source] std::io::Error),

    #[error("couldn't create main window")]
    CreateMainWindow(#[source] slint::PlatformError),

    #[error("couldn't initialize graphics engine")]
    InitGraphics(#[source] GraphicsError),

    #[error("couldn't center main window")]
    CenterMainWindow(#[source] self::ui::PlatformError),

    #[error("couldn't show main window")]
    ShowMainWindow(#[source] slint::PlatformError),

    #[error("couldn't create {0}")]
    CreateKernelLog(PathBuf, #[source] std::io::Error),

    #[error("couldn't build graphics engine")]
    BuildGraphicsEngine(#[source] GraphicsError),

    #[error("couldn't start VMM for {0}")]
    StartVmm(PathBuf, #[source] VmmError),

    #[error("thread for vCPU #{0} was stopped unexpectedly")]
    CpuThread(#[source] CpuError),

    #[error("vCPU #{0} panicked, see {1} for more information")]
    CpuPanic(usize, PathBuf),

    #[error("couldn't read debugger connection")]
    ReadDebuggerSocket(#[source] std::io::Error),

    #[error("couldn't dispatch debugger requests")]
    DispatchDebugger(#[source] GdbError),

    #[error("couldn't write debugger connection")]
    WriteDebuggerSocket(#[source] std::io::Error),
}
