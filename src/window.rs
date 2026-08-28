use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc, OnceLock};

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, OwnedDisplayHandle};
use winit::window::{Fullscreen, Window, WindowAttributes, WindowId};

use crate::style::StyleSheet;
use crate::ui::{self, Widget};

/// winit's `EventLoop` is `!Send`. We host a single shared loop on a dedicated
/// thread, created lazily. It is confined to that thread for its lifetime.
struct SendEventLoop(EventLoop<()>);

unsafe impl Send for SendEventLoop {}

impl SendEventLoop {
    fn run(self, app: &mut impl ApplicationHandler) -> Result<(), winit::error::EventLoopError> {
        self.0.run_app(app)
    }
}

pub type SignalCallback = unsafe extern "C" fn(name: *const std::os::raw::c_char, userdata: *mut std::os::raw::c_void);

/// A signal callback plus userdata, made `Send` so it can travel through the
/// command channel to the event-loop thread. The userdata pointer must remain
/// valid for the window's lifetime (the C# side keeps it alive).
#[derive(Clone, Copy)]
pub struct SignalCb(pub SignalCallback, pub *mut std::os::raw::c_void);

unsafe impl Send for SignalCb {}

#[derive(Clone)]
pub struct ModulePayload {
    pub tree: Widget,
    pub sheet: StyleSheet,
}

pub enum Command {
    SetTitle(String),
    SetFullscreen(bool),
    SetSize(u32, u32),
    Close,
    LoadModule(ModulePayload),
    SetSignalCallback(Option<SignalCb>),
}

struct CreateRequest {
    size: (u32, u32),
    cmd_rx: mpsc::Receiver<Command>,
    created: Arc<AtomicBool>,
    should_close: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
}

struct UiState {
    tree: Widget,
    sheet: StyleSheet,
}

struct WindowData {
    window: Arc<Window>,
    surface: Option<Surface<OwnedDisplayHandle, Arc<Window>>>,
    cmd_rx: mpsc::Receiver<Command>,
    should_close: Arc<AtomicBool>,
    ui: Option<UiState>,
    signal_cb: Option<SignalCb>,
    cursor: (i32, i32),
}

struct App {
    context: Context<OwnedDisplayHandle>,
    windows: HashMap<WindowId, WindowData>,
    create_rx: mpsc::Receiver<CreateRequest>,
    pending_creates: Vec<CreateRequest>,
}

fn resize_surface(width: u32, height: u32, surface: &mut Surface<OwnedDisplayHandle, Arc<Window>>) {
    if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
        let _ = surface.resize(w, h);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let mut close_requested = false;
        if let Some(data) = self.windows.get_mut(&id) {
            match event {
                WindowEvent::CloseRequested => {
                    data.should_close.store(true, Ordering::Relaxed);
                    close_requested = true;
                }
                WindowEvent::Resized(size) => {
                    if let Some(ref mut surface) = data.surface {
                        resize_surface(size.width, size.height, surface);
                    }
                    let _ = data.window.request_redraw();
                }
                WindowEvent::CursorMoved {
                    position: PhysicalPosition { x, y },
                    ..
                } => {
                    data.cursor = (x as i32, y as i32);
                }
                WindowEvent::MouseInput {
                    state, button, ..
                } => {
                    if state == ElementState::Pressed && button == MouseButton::Left {
                        self.handle_click(id);
                    }
                }
                WindowEvent::RedrawRequested => {
                    self.drain_commands(id);
                    let closed = self
                        .windows
                        .get(&id)
                        .map(|d| d.should_close.load(Ordering::Relaxed))
                        .unwrap_or(false);
                    if closed {
                        self.close_window(id);
                    }
                    self.render(id);
                }
                _ => {}
            }
        }

        if close_requested {
            self.close_window(id);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(req) = self.create_rx.try_recv() {
            self.pending_creates.push(req);
        }
        let creates = std::mem::take(&mut self.pending_creates);
        for req in creates {
            self.create_window(event_loop, req);
        }
        for data in self.windows.values() {
            let _ = data.window.request_redraw();
        }
    }
}

impl App {
    fn close_window(&mut self, id: WindowId) {
        if let Some(data) = self.windows.remove(&id) {
            data.should_close.store(true, Ordering::Relaxed);
            drop(data.window);
        }
    }

    fn handle_click(&mut self, id: WindowId) {
        let Some(data) = self.windows.get_mut(&id) else { return };
        let Some(ref ui) = data.ui else { return };
        let (w, h) = (data.window.inner_size().width, data.window.inner_size().height);
        let (cx, cy) = data.cursor;
        if cx < 0 || cy < 0 {
            return;
        }
        let placed = ui::layout(&ui.tree, &ui.sheet, w, h);
        let signal = ui::hit_test(&placed, cx as u32, cy as u32);
        if let Some(sig) = signal {
            if let Some(SignalCb(cb, userdata)) = data.signal_cb {
                let c_sig = std::ffi::CString::new(sig).unwrap_or_default();
                unsafe {
                    cb(c_sig.as_ptr(), userdata);
                }
            }
        }
    }

    fn create_window(&mut self, event_loop: &ActiveEventLoop, req: CreateRequest) {
        let (w, h) = req.size;
        let attrs = WindowAttributes::default()
            .with_title("ULib Window")
            .with_inner_size(winit::dpi::PhysicalSize::new(w, h));
        match event_loop.create_window(attrs) {
            Ok(window) => {
                let window = Arc::new(window);
                let mut surface = Surface::new(&self.context, window.clone()).unwrap();
                let size = window.inner_size();
                resize_surface(size.width, size.height, &mut surface);

                let id = window.id();
                self.windows.insert(
                    id,
                    WindowData {
                        window,
                        surface: Some(surface),
                        cmd_rx: req.cmd_rx,
                        should_close: req.should_close.clone(),
                        ui: None,
                        signal_cb: None,
                        cursor: (0, 0),
                    },
                );
                req.created.store(true, Ordering::Relaxed);
            }
            Err(e) => {
                eprintln!("[ulib] failed to create window: {e}");
                req.failed.store(true, Ordering::Relaxed);
            }
        }
    }

    fn drain_commands(&mut self, id: WindowId) {
        let Some(data) = self.windows.get_mut(&id) else { return };
        while let Ok(cmd) = data.cmd_rx.try_recv() {
            match cmd {
                Command::SetTitle(t) => data.window.set_title(&t),
                Command::SetFullscreen(fs) => {
                    if fs {
                        let _ = data.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    } else {
                        let _ = data.window.set_fullscreen(None);
                    }
                }
                Command::SetSize(w, h) => {
                    let _ = data.window.request_inner_size(winit::dpi::PhysicalSize::new(w, h));
                }
                Command::Close => data.should_close.store(true, Ordering::Relaxed),
                Command::LoadModule(payload) => {
                    data.ui = Some(UiState {
                        tree: payload.tree,
                        sheet: payload.sheet,
                    });
                }
                Command::SetSignalCallback(cb) => data.signal_cb = cb,
            }
        }
    }

    fn render(&mut self, id: WindowId) {
        let Some(data) = self.windows.get_mut(&id) else { return };
        let Some(ref mut surface) = data.surface else { return };
        let Ok(mut buffer) = surface.buffer_mut() else { return };
        let w = buffer.width().get();
        let h = buffer.height().get();

        if let Some(ref ui) = data.ui {
            let placed = ui::layout(&ui.tree, &ui.sheet, w, h);
            ui::render(&placed, &mut buffer, w, h);
        } else {
            // Fallback gradient if no UI loaded.
            for y in 0..h {
                for x in 0..w {
                    let red = (x / 8) % 256;
                    let green = (y / 8) % 256;
                    let blue = ((x * y) / 16) % 256;
                    buffer[(y * w + x) as usize] = (blue << 16) | (green << 8) | red;
                }
            }
        }
        let _ = buffer.present();
    }
}

/// Shared event-loop singleton state.
struct Shared {
    create_tx: mpsc::Sender<CreateRequest>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();

fn start_shared_loop() -> &'static Shared {
    SHARED.get_or_init(|| {
        let event_loop = EventLoop::new().expect("failed to create event loop");
        let owned_display = event_loop.owned_display_handle();
        let send_event_loop = SendEventLoop(event_loop);

        let (create_tx, create_rx) = mpsc::channel::<CreateRequest>();

        std::thread::Builder::new()
            .name("ulib-event-loop".into())
            .spawn(move || {
                let mut app = App {
                    context: Context::new(owned_display).unwrap(),
                    windows: HashMap::new(),
                    create_rx,
                    pending_creates: Vec::new(),
                };
                send_event_loop.run(&mut app).unwrap();
            })
            .expect("failed to start event-loop thread");

        Shared { create_tx }
    })
}

pub struct WindowHandle {
    cmd_tx: mpsc::Sender<Command>,
    should_close: Arc<AtomicBool>,
    size: (u32, u32),
}

impl WindowHandle {
    pub fn new(width: u32, height: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let shared = start_shared_loop();

        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let created = Arc::new(AtomicBool::new(false));
        let should_close = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));

        shared
            .create_tx
            .send(CreateRequest {
                size: (width, height),
                cmd_rx,
                created: created.clone(),
                should_close: should_close.clone(),
                failed: failed.clone(),
            })
            .map_err(|_| "event loop is down".to_string())?;

        let start = std::time::Instant::now();
        loop {
            if created.load(Ordering::Relaxed) {
                break;
            }
            if failed.load(Ordering::Relaxed) {
                return Err("failed to create window".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
            if start.elapsed() > std::time::Duration::from_secs(5) {
                return Err("timeout waiting for window creation".into());
            }
        }

        Ok(WindowHandle {
            cmd_tx,
            should_close,
            size: (width, height),
        })
    }

    pub fn set_title(&self, title: &str) {
        let _ = self.cmd_tx.send(Command::SetTitle(title.to_string()));
    }

    pub fn set_fullscreen(&self, fullscreen: bool) {
        let _ = self.cmd_tx.send(Command::SetFullscreen(fullscreen));
    }

    pub fn set_size(&self, width: u32, height: u32) {
        let _ = self.cmd_tx.send(Command::SetSize(width, height));
    }

    pub fn width(&self) -> u32 {
        self.size.0
    }

    pub fn height(&self) -> u32 {
        self.size.1
    }

    pub fn poll_close(&self) -> bool {
        self.should_close.load(Ordering::Relaxed)
    }

    pub fn close(&self) {
        let _ = self.cmd_tx.send(Command::Close);
    }

    pub fn load_module(&self, payload: ModulePayload) {
        let _ = self.cmd_tx.send(Command::LoadModule(payload));
    }

    pub fn set_signal_callback(&self, cb: Option<SignalCb>) {
        let _ = self.cmd_tx.send(Command::SetSignalCallback(cb));
    }
}
