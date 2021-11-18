#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crossbeam_channel::Sender;
use crossbeam_channel::TryRecvError::*;
use late_init::LateInit;
use native_windows_gui as nwg;
use once_cell::sync::Lazy;
use os_str_bytes::OsStrBytes;
use single_instance::SingleInstance;
use std::cell::RefCell;
use std::io::Write;
use std::mem;
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::ptr::NonNull;
use std::time::Duration;
use std::time::SystemTime;
use winapi::shared::windef;
use winapi::um::dwmapi;
use winapi::um::winuser;
use winapi::um::wingdi;

static ICON: &'static [u8] = include_bytes!("../icon.png");

fn bitmap_info_header(width: i32, height: i32) -> wingdi::BITMAPINFOHEADER {
    wingdi::BITMAPINFOHEADER {
        biSize: mem::size_of::<wingdi::BITMAPINFOHEADER>() as _,
        biWidth: width,
        biHeight: height,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: wingdi::BI_RGB,
        ..unsafe { mem::zeroed() }
    }
}

trait Ptr {
    type T;
}

impl<T> Ptr for *mut T {
    type T = T;
}

type NonNullHWND = ptr::NonNull<<windef::HWND as Ptr>::T>;

fn get_foregound_window_handle() -> Result<NonNullHWND, NonNullHWND> {
    let foreground = unsafe { winuser::GetForegroundWindow() };
    NonNull::new(foreground).ok_or_else(get_desktop_window_handle)
}

fn get_desktop_window_handle() -> NonNullHWND {
    // Safety: GetDesktopWindow never returns null.
    unsafe { NonNull::new_unchecked(winuser::GetDesktopWindow()) }
}

fn get_window_rect(mut use_foreground: bool) -> windef::RECT {
    let mut rect = unsafe { mem::zeroed() };
    let hwnd = if mem::take(&mut use_foreground) {
        match get_foregound_window_handle() {
            Ok(hwnd) if unsafe {
                dwmapi::DwmGetWindowAttribute(
                hwnd.as_ptr(),
                dwmapi::DWMWA_EXTENDED_FRAME_BOUNDS,
                &mut rect as *mut _ as *mut _,
                mem::size_of::<windef::RECT>() as _,
            ) == 0 } => {
                use_foreground = true;
                hwnd
            }
            Err(hwnd) => hwnd,
            Ok(_) => get_desktop_window_handle(),
        }
    }
    else {
        get_desktop_window_handle()
    };
    if !use_foreground {
        unsafe { winuser::GetWindowRect(hwnd.as_ptr(), &mut rect) };
    }
    rect
}

struct Bitmap {
    buf: Vec<u8>,
    width: u32,
    height: u32,
}

#[inline(never)]
fn capture_screenshot(use_foreground: bool) -> Bitmap {
    let rect = get_window_rect(use_foreground);
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);

    let window_ctx = unsafe { winuser::GetDC(ptr::null_mut()) };
    let memory_ctx = unsafe { wingdi::CreateCompatibleDC(window_ctx) };

    let window_bmp = unsafe { wingdi::CreateCompatibleBitmap(window_ctx, width, height) };
    let old_bmp = unsafe { wingdi::SelectObject(memory_ctx, window_bmp.cast()) };
    let bmp_info = &mut bitmap_info_header(width, height) as *mut wingdi::BITMAPINFOHEADER;
    let mut buf = vec![0u8; (4 * width * height) as usize];

    unsafe { wingdi::BitBlt(
        memory_ctx,
        0,
        0,
        width,
        height,
        window_ctx,
        rect.left,
        rect.top,
        wingdi::SRCCOPY,
    )};

    unsafe { wingdi::GetDIBits(
        memory_ctx, 
        window_bmp, 
        0, 
        height as _, 
        buf.as_mut_ptr().cast(),
        bmp_info.cast(),
        wingdi::DIB_RGB_COLORS,
    )};

    unsafe {
        wingdi::SelectObject(memory_ctx, old_bmp);
        wingdi::DeleteObject(window_bmp.cast());
        wingdi::DeleteDC(memory_ctx);
        winuser::ReleaseDC(ptr::null_mut(), window_ctx);
    }

    Bitmap {
        buf,
        width: width as _,
        height: height as _,
    }
}

struct KeyState {
    alt: bool,
    prtsc: bool,
    prtsc_old: bool,
}

impl KeyState {
    fn retrieve() -> Self {
        let key_down = |key| unsafe { winuser::GetAsyncKeyState(key) < 0 };
        Self {
            alt: key_down(winuser::VK_MENU),
            prtsc: key_down(winuser::VK_SNAPSHOT),
            prtsc_old: false,
        }
    }

    fn update(&mut self) {
        let new_st = Self::retrieve();
        self.alt = new_st.alt;
        self.prtsc_old = mem::replace(&mut self.prtsc, new_st.prtsc);
    }

    fn prtsc_pressed(&self) -> bool {
        self.prtsc && !self.prtsc_old
    }
}

fn write_name(mut bytes: &mut [u8], index: u128) -> usize {
    let start = bytes.as_ptr() as usize;
    write!(bytes, "{}", index).ok();
    bytes.write(b".png").ok();
    bytes.as_ptr() as usize - start
}

fn capture_and_save(bitmap: Bitmap, mut path: PathBuf, index: u128) {
    let Bitmap { buf, width, height } = bitmap;
    let mut name_stack = [0; 43];
    let len = write_name(&mut name_stack, index);
    let name = match std::str::from_utf8(&name_stack[..len]) {
        Ok(n) => n,
        Err(_) => return,
    };
    path.push(name);
    let mut image = image::ImageBuffer::from_vec(width, height, buf).unwrap();
    image::imageops::rotate180_in_place(&mut image);
    image::imageops::flip_horizontal_in_place(&mut image);
    let image = image::DynamicImage::ImageBgra8(image);
    image.to_rgba8().save(path).ok();
}

static REG_RUN: &str = r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run";
static CURRENT_EXE: Lazy<PathBuf> = Lazy::new(|| std::env::current_exe().and_then(std::fs::canonicalize).unwrap());

fn reg_add() {
    let device_namespace_removed = &CURRENT_EXE.to_raw_bytes()[4..];
    let current_exe_fs = OsStrBytes::from_raw_bytes(device_namespace_removed).unwrap();
    Command::new("reg")
        .args(["add", REG_RUN, "/v", "SilentShot", "/t", "REG_SZ", "/d"])
        .arg(current_exe_fs)
        .arg("/f")
        .spawn().ok();
}

fn reg_delete() {
    Command::new("reg")
        .args(["delete", REG_RUN, "/v", "SilentShot", "/f"])
        .spawn().ok();
}

fn reg_query() -> bool {
    Command::new("reg")
        .args(["query", REG_RUN, "/v", "SilentShot"])
        .status().map_or(false, |status| status.success())
}

#[derive(LateInit)]
struct ConfigAndTray {
    window: nwg::Window,
    icon: nwg::Icon,
    tray: nwg::TrayNotification,
    tray_menu: nwg::Menu,
    tray_item_startup_flag: nwg::MenuItem,
    tray_item_open_dir: nwg::MenuItem,
    tray_item_change_dir: nwg::MenuItem,
    tray_item_exit: nwg::MenuItem,
    open_dir_dialog: nwg::FileDialog,
    sender: Sender<PathBuf>,
    path: RefCell<PathBuf>,
}

static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| dirs::config_dir().unwrap().join("silent_config"));
static DEST_PATH: Lazy<PathBuf> = Lazy::new(|| CONFIG_PATH.join("dest"));
static DEFAULT_DEST: Lazy<PathBuf> = Lazy::new(|| dirs::picture_dir().unwrap().join("Screenshots"));

impl ConfigAndTray {
    fn new(sender: Sender<PathBuf>) -> Self {
        let path = match std::fs::read_to_string(&*DEST_PATH) {
            Ok(s) => PathBuf::from(s),
            _ => DEFAULT_DEST.clone(),
        };
        sender.send(path.clone()).ok();
        ConfigAndTrayLateInit::default()
            .sender(sender)
            .path(RefCell::new(path))
            .finish()
    }

    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn toggle_startup(&self) {
        let flag = self.tray_item_startup_flag.checked();
        let real = reg_query();
        match real {
            true if flag => reg_delete(),
            false if !flag => reg_add(),
            _ => (),
        }
        self.tray_item_startup_flag.set_checked(!flag);
    }

    fn open_dir(&self) {
        Command::new("explorer")
            .arg(self.path.borrow().as_path())
            .status()
            .ok();
    }

    fn change_dir(&self) {
        self.open_dir_dialog.run(Some(&self.window));
        match self.open_dir_dialog.get_selected_item().map(PathBuf::from) {
            Ok(dir) => {
                if &dir == &*DEFAULT_DEST {
                    std::fs::remove_file(&*DEST_PATH).ok();
                    std::fs::remove_dir(&*CONFIG_PATH).ok();
                }
                else {
                    std::fs::create_dir(&*CONFIG_PATH).ok();
                    std::fs::write(&*DEST_PATH, dir.to_raw_bytes()).ok();
                }
                self.sender.send(dir.clone()).ok();
                *self.path.borrow_mut() = dir;
            },
            Err(_) => (),
        }
    }
    
    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

mod ui {
    use native_windows_gui as nwg;
    use super::ConfigAndTray;
    use std::rc::Rc;
    use std::cell::RefCell;
    use std::ops::Deref;

    pub(super) struct ConfigAndTrayUi {
        inner: Rc<ConfigAndTray>,
        default_handler: RefCell<Vec<nwg::EventHandler>>
    }

    impl nwg::NativeUi<ConfigAndTrayUi> for ConfigAndTray {
        fn build_ui(mut data: ConfigAndTray) -> Result<ConfigAndTrayUi, nwg::NwgError> {
            use nwg::Event as E;

            // Resources
            data.icon = nwg::Bitmap::from_bin(super::ICON)?.copy_as_icon();
            
            // Controls
            nwg::Window::builder()
                .title("Silent Shot")
                .flags(nwg::WindowFlags::WINDOW)
                .build(&mut data.window)?;

            nwg::TrayNotification::builder()
                .parent(&data.window)
                .icon(Some(&data.icon))
                .tip(Some("Silent Shot"))
                .build(&mut data.tray)?;

            nwg::Menu::builder()
                .popup(true)
                .parent(&data.window)
                .build(&mut data.tray_menu)?;

            nwg::MenuItem::builder()
                .text("Автозапуск")
                .check(super::reg_query())
                .parent(&data.tray_menu)
                .build(&mut data.tray_item_startup_flag)?;

            nwg::MenuItem::builder()
                .text("Открыть папку")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item_open_dir)?;

            nwg::MenuItem::builder()
                .text("Изменить путь...")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item_change_dir)?;

            nwg::MenuItem::builder()
                .text("Выход")
                .parent(&data.tray_menu)
                .build(&mut data.tray_item_exit)?;

            // Dialogs
            nwg::FileDialog::builder()
                .title("Укажите путь к папке назначения")
                .action(nwg::FileDialogAction::OpenDirectory)
                .build(&mut data.open_dir_dialog)?;

            // Wrap-up
            let ui = ConfigAndTrayUi {
                inner: Rc::new(data),
                default_handler: Default::default(),
            };

            // Events
            let evt_ui = Rc::downgrade(&ui.inner);
            let handle_events = move |evt, _evt_data, handle| {
                if let Some(evt_ui) = evt_ui.upgrade() {
                    match evt {
                        E::OnContextMenu => 
                            if handle == evt_ui.tray {
                                evt_ui.show_menu();
                            }
                        E::OnMenuItemSelected =>
                            if handle == evt_ui.tray_item_startup_flag {
                                evt_ui.toggle_startup();
                            }
                            else if handle == evt_ui.tray_item_open_dir {
                                evt_ui.open_dir();
                            }
                            else if handle == evt_ui.tray_item_change_dir {
                                evt_ui.change_dir();
                            }
                            else if handle == evt_ui.tray_item_exit {
                                evt_ui.exit();
                            },
                        _ => {}
                    }
                }
            };

            ui.default_handler.borrow_mut().push(
                nwg::full_bind_event_handler(&ui.window.handle, handle_events)
            );

            return Ok(ui);
        }
    }

    impl Drop for ConfigAndTrayUi {
        /// To make sure that everything is freed without issues, the default handler must be unbound.
        fn drop(&mut self) {
            let mut handlers = self.default_handler.borrow_mut();
            for handler in handlers.drain(..) {
                nwg::unbind_event_handler(&handler);
            }
        }
    }

    impl Deref for ConfigAndTrayUi {
        type Target = ConfigAndTray;

        fn deref(&self) -> &ConfigAndTray {
            &self.inner
        }
    }
}

fn run_ui_thread(sender: Sender<PathBuf>) {
    std::thread::spawn(move || {
        nwg::init().expect("Failed to init Native Windows GUI");
        let _ui = nwg::NativeUi::build_ui(ConfigAndTray::new(sender)).expect("Failed to build UI");
        nwg::dispatch_thread_events();
    });
}

fn main() {
    let guid_new = SingleInstance::new("3a9cfdf2-0c48-4e70-bb97-f2a4220c8da1").unwrap();
    let guid_old = SingleInstance::new("{3A9CFDF2-0C48-4E70-BB97-F2A4220C8DA1}").unwrap();
    assert!(guid_new.is_single());
    assert!(guid_old.is_single());
    let mut keys = KeyState::retrieve();
    let num_cpus = num_cpus::get();
    let num_threads = num_cpus.saturating_sub(1).max(1);
    let (tx, rx) = crossbeam_channel::bounded(2 * num_threads);
    let (ui_tx, ui_rx) = crossbeam_channel::bounded(0);
    run_ui_thread(ui_tx);
    let threads: Vec<_> = vec![rx; num_threads].into_iter().map(|rx|
        std::thread::spawn(move || {
            while let Ok((bitmap, path, index)) = rx.recv() {
                capture_and_save(bitmap, path, index);
            }
        }))
        .collect();
    const SLEEP_TIME: Duration = Duration::from_millis(20);
    let mut path = PathBuf::new();
    loop {
        match ui_rx.try_recv() {
            Ok(p) => path = p,
            Err(Empty) => (),
            Err(Disconnected) => break,
        }
        keys.update();
        if keys.prtsc_pressed() {
            // Shouldn't panic since UNIX_EPOCH is the mininum of SystemTime
            let index = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
            let bitmap = capture_screenshot(keys.alt);
            tx.send((bitmap, path.clone(), index)).ok();
        }
        keys.update();
        std::thread::sleep(SLEEP_TIME);
    }
    drop(tx);
    for t in threads {
        t.join().ok();
    }
}
