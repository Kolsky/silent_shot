use winapi::{
    um::winuser::{GetForegroundWindow, GetWindowRect},
    shared::{
        windef::{RECT},
    },
};
use std::{
    env,
    mem::zeroed,
    io,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
    fs,
    ffi::OsStr,
    convert::AsRef,
    path::{PathBuf, Path},
    process::Command
};
use scrap::Frame;

pub fn configure_startup(enabled: bool) {
    let exe_path = format!(r#""{}""#, env::current_exe().unwrap().to_str().unwrap());
    fn run(cmd: &str, args: &[&str]) {
        Command::new("reg")
        .arg(dbg!(cmd))
        .arg(r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Run")
        .args(&["/v", "SilentShot"])
        .args(args)
        .arg("/f")
        .output().unwrap();
    }
    if enabled {
        run("add", &["/t", "REG_SZ", "/d", exe_path.as_str()]);
    }
    else {
        run("delete", &[]);
    }
}

fn clamp(value: i32, min: i32, max: i32) -> Option<i32> {
    if min > max { None }
    else if value <= min { Some(min) }
    else if value >= max { Some(max) }
    else { Some(value) }
}

pub fn convert_all_tga_to_png<T: AsRef<Path>>(dir: T, preserve_tga: bool) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if convert_tga_to_png(&path, preserve_tga) {
            thread::sleep(Duration::from_millis(100));
        }
    }
    Ok(())
}

pub fn convert_tga_to_png<T: AsRef<Path>>(path: T, preserve_tga: bool) -> bool {
    let path = PathBuf::from(path.as_ref());
    if let Some(x) = path.extension() {
        if x == OsStr::new("tga") {
            if let Ok(tga) = image::open(&path) {
                let mut new_path = path.clone();
                new_path.set_extension("png");
                tga.save_with_format(&new_path, image::ImageFormat::Png).unwrap_or_default();
                if !preserve_tga { 
                    fs::remove_file(&path).unwrap_or_default();
                }
                return true
            }
        }
    }
    false
}

pub fn crop_full_frame(buf: &mut Vec<u8>, frame: Frame, width: usize, height: usize) {
    let stride = frame.len() / height;
    let rows = frame.chunks(stride);
    buf.clear();
    for row in rows {
        let row = &row[..4 * width];
        buf.extend_from_slice(row);
    }
}

pub fn crop_frame_and_return_dims(buf: &mut Vec<u8>, frame: Frame, rect: RECT, width: usize, height: usize) -> (usize, usize) {
    let stride = frame.len() / height;
    let top = clamp(rect.top, 0, height as i32).unwrap() as usize;
    let bottom = clamp(rect.bottom - 7, 0, height as i32).unwrap() as usize;
    let left = clamp(rect.left + 7, 0, width as i32).unwrap() as usize * 4;
    let right = clamp(rect.right - 7, 0, width as i32).unwrap() as usize * 4;
    let rows = frame.chunks(stride).into_iter().take(bottom).skip(top);
    buf.clear();
    for row in rows {
        let row = &row[left..right];
        buf.extend_from_slice(row);
    }
    ((right - left) / 4, bottom - top)
}

pub fn get_active_window_rect() -> Option<RECT> {
    unsafe {
        let hwnd = GetForegroundWindow();
        let mut rect = zeroed::<RECT>();
        match GetWindowRect(hwnd, &mut rect) {
            0 => {
                None
            },
            _ => Some(rect)
        }
    }
}

pub fn get_user_default_gallery_dir() -> String {
    format!("{}/Pictures/Screenshots/", env::var("USERPROFILE").unwrap())
}

pub fn save_tga(save_folder: &str, buffer: &[u8], width: usize, height: usize) -> PathBuf {
    let mut path = PathBuf::from(save_folder);
    path.push(format!("{}.tga", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros()));
    image::save_buffer_with_format(
        &path,
        buffer,
        width as u32,
        height as u32,
        image::ColorType::Bgra8,
        image::ImageFormat::Tga)
        .unwrap_or_else(|e| -> () { println!("{}", e) });
    path
}