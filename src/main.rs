#![windows_subsystem="windows"]
mod config;
mod keyboard;
mod sysnio;

use config::Config;
use keyboard::{KeyEvent, retrieve_keys};
use sysnio::{
    convert_tga_to_png,
    convert_all_tga_to_png,
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
use single_instance::SingleInstance;

fn get_gallery_dir() -> String {
    get_user_default_gallery_dir()
}

fn main() {
    let guid_new = SingleInstance::new("3a9cfdf2-0c48-4e70-bb97-f2a4220c8da1").unwrap();
    let guid_old = SingleInstance::new("{3A9CFDF2-0C48-4E70-BB97-F2A4220C8DA1}").unwrap();
    assert!(guid_new.is_single());
    assert!(guid_old.is_single());
    let save_folder = get_gallery_dir();
    let str_clone = save_folder.clone();
    let save_folder = save_folder.as_str();
    let _ = dbg!(Config::open_or_create_default("config.ron"));
    std::fs::create_dir_all(save_folder).unwrap();
    thread::spawn(move || {
        let save_folder = str_clone.as_str();
        loop {
            convert_all_tga_to_png(save_folder, false).unwrap_or_default();
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
        //     dbg!("Esc");
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
                        dbg!("Skip frame");
                        thread::sleep(frame_time);
                        continue
                    }
                    _ => break
                };
                if keys.vk_menu.is_down() {
                    dbg!("Alt + PrtScn");
                    if let Some(rect) = get_active_window_rect() {
                        let (w, h) = crop_frame_and_return_dims(&mut buf, frame, rect, width, height);
                        save_tga(save_folder, buf.as_slice(), w, h);
                    }
                    else {
                        save_tga(save_folder, &frame, width, height);
                    }
                }
                else {
                    dbg!("PrtScn");
                    save_tga(save_folder, &frame, width, height);
                }
                snapshot_evt = KeyEvent::Down;
            }
            _ => {}
        }
        thread::sleep(frame_time);
    }
}
