#![cfg_attr(not(debug_assertions), windows_subsystem="windows")]
mod config;
mod keyboard;
mod sysnio;

use config::{Config, Opt::Custom, Opt::Default, ImageFormat};
use keyboard::{KeyEvent, retrieve_keys};
use sysnio::{
    configure_startup,
    convert_tga_to_png,
    convert_all_tga_to_png,
    crop_full_frame,
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
    sync::mpsc::{channel, Receiver},
    path::Path,
};
use single_instance::SingleInstance;

fn get_gallery_dir(config: &Config) -> String {
    match &config.save_folder {
        Custom(folder) => { folder.clone() }
        Default => { get_user_default_gallery_dir() }
    }
}

fn open_image_conversion_thread<'a>(save_folder: String, preserve_tga: bool, rx: Receiver<std::path::PathBuf>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let save_folder = save_folder.as_str();
        convert_all_tga_to_png(save_folder, true).unwrap_or_default();
        loop {
            let tga_path = rx.recv().unwrap();
            convert_tga_to_png(tga_path, preserve_tga);
        }
    })
}

fn main() {
    let guid_new = SingleInstance::new("3a9cfdf2-0c48-4e70-bb97-f2a4220c8da1").unwrap();
    let guid_old = SingleInstance::new("{3A9CFDF2-0C48-4E70-BB97-F2A4220C8DA1}").unwrap();
    assert!(guid_new.is_single());
    assert!(guid_old.is_single());
    let cwd = dbg!(std::env::current_exe().ok());
    let config_path = cwd.as_ref().map(Path::new).and_then(Path::parent).map(|p| p.join("config.ron")).unwrap();
    let config = Config::open_or_create_default(config_path);
    configure_startup(dbg!(config.enable_startup));
    let save_folder = get_gallery_dir(&config);
    let str_clone = save_folder.clone();
    let save_folder = save_folder.as_str();
    std::fs::create_dir_all(save_folder).unwrap();
    let (tx, rx) = channel();
    match config.image_format {
        ImageFormat::Both => {
            open_image_conversion_thread(str_clone, true, rx);
        },
        ImageFormat::Png => {
            open_image_conversion_thread(str_clone, false, rx);
        },
        ImageFormat::Tga => {},
    }
    let frame_time = Duration::from_secs_f32(1.0 / 60.0);
    let display = Display::primary().unwrap();
    let mut capturer = Capturer::new(display).unwrap();
    let width = capturer.width();
    let height = capturer.height();
    let mut buf : Vec<u8> = Vec::with_capacity(width * height * 4);
    let mut snapshot_evt = KeyEvent::Up;
    let should_send = config.image_format != ImageFormat::Tga;
    loop {
        let keys = retrieve_keys();
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
                let path = 
                    if keys.vk_menu.is_down() {
                        dbg!("Alt + PrtScn");
                        if let Some(rect) = get_active_window_rect() {
                            let (w, h) = crop_frame_and_return_dims(&mut buf, frame, rect, width, height);
                            save_tga(save_folder, buf.as_slice(), w, h)
                        }
                        else {
                            crop_full_frame(&mut buf, frame, width, height);
                            save_tga(save_folder, buf.as_slice(), width, height)
                        }
                    }
                    else {
                        dbg!("PrtScn");
                        crop_full_frame(&mut buf, frame, width, height);
                        save_tga(save_folder, buf.as_slice(), width, height)
                    };
                if should_send {
                    tx.send(path).unwrap();
                }
                snapshot_evt = KeyEvent::Down;
            }
            _ => {}
        }
        thread::sleep(frame_time);
    }
}
