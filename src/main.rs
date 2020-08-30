#![windows_subsystem="windows"]
mod keyboard;
mod sysnio;

use keyboard::{KeyEvent, VirtualKey, retrieve_keys};
use sysnio::{
    convert_tga_to_png,
    crop_frame_and_return_dims,
    get_active_window_rect,
    get_user_default_gallery_dir,
    save_tga,
};

use scrap::{Display, Capturer};
use std::{
    io::ErrorKind::WouldBlock,
    thread,
    time::Duration,
    path::PathBuf,
};

fn get_gallery_dir() -> String {
    get_user_default_gallery_dir()
}

fn main() {
    let save_folder = get_gallery_dir();
    thread::spawn(move || {
        let save_folder = PathBuf::from(get_gallery_dir());
        loop {
            convert_tga_to_png(&save_folder).unwrap_or_default();
            thread::sleep(Duration::from_secs(1));
        }
    });
    let frame_time = Duration::from_secs_f32(1.0 / 60.0);
    let display = Display::primary().unwrap();
    let mut capturer = Capturer::new(display).unwrap();
    let width = capturer.width();
    let height = capturer.height();
    let mut buf : Vec<u8> = Vec::with_capacity(width * height * 4);
    let mut snapshot_evt = KeyEvent::Up;
    loop {
        let keys = retrieve_keys();
        // if keys.vk_escape.is_down() {
        //     println!("Esc");
        //     return;
        // }
        match keys.vk_snapshot.is_down() {
            true if snapshot_evt == KeyEvent::Up => {
                snapshot_evt = KeyEvent::Fire
            }
            false => {
                snapshot_evt = KeyEvent::Up
            }
            _ => {}
        }
        match snapshot_evt {
            KeyEvent::Fire => {
                let frame = match capturer.frame() {
                    Ok(f) => f,
                    Err(e) if e.kind() == WouldBlock => {
                        println!("Skip frame");
                        thread::sleep(frame_time);
                        continue
                    }
                    _ => break
                };
                if keys.vk_menu.is_down() {
                    println!("Alt + PrtScn");
                    if let Some(rect) = get_active_window_rect() {
                        let (w, h) = crop_frame_and_return_dims(&mut buf, frame, rect, width, height);
                        save_tga(save_folder.as_str(), buf.as_slice(), w, h);
                    }
                    else {
                        save_tga(save_folder.as_str(), &frame, width, height);
                    }
                }
                else {
                    println!("PrtScn");
                    save_tga(save_folder.as_str(), &frame, width, height);
                }
                snapshot_evt = KeyEvent::Down;
            }
            _ => {}
        }
        thread::sleep(frame_time);
    }
}
