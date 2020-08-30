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
    path::PathBuf,
};
use scrap::Frame;

fn clamp(value: i32, min: i32, max: i32) -> Option<i32> {
    if min > max { None }
    else if value <= min { Some(min) }
    else if value >= max { Some(max) }
    else { Some(value) }
}

pub fn convert_tga_to_png(dir: &PathBuf) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let mut path = entry.path();
        if let Some(x) = path.extension() {
            if x == OsStr::new("tga") {
                if let Ok(tga) = image::open(&path) {
                    fs::remove_file(&path).unwrap_or_default();
                    path.set_extension("png");
                    tga.save_with_format(&path, image::ImageFormat::Png).unwrap_or_default();
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
    Ok(())
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

pub fn save_tga(save_folder: &str, buffer: &[u8], width: usize, height: usize) -> () {
    image::save_buffer_with_format(
        format!("{}{}.tga", save_folder, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros()),
        buffer,
        width as u32,
        height as u32,
        image::ColorType::Bgra8,
        image::ImageFormat::Tga)
        .unwrap_or_else(|e| -> () { println!("{}", e) });
}