use std::{
    convert::AsRef,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use ron::{from_str, ser::to_string_pretty};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Opt<T> {
    Default,
    Custom(T)
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    Png,
    Tga,
    Both,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    save_folder: Opt<String>,
    image_format: ImageFormat,
    enable_startup: bool,
}

impl Config {
    pub fn open_or_create_default<T: AsRef<Path>>(path: T) -> Self {
        if let Ok(mut file) = dbg!(File::open(&path)) {
            let mut contents = String::new();
            if let Ok(_) = file.read_to_string(&mut contents) {
                dbg!(&contents);
                if let Ok(config) = from_str(&contents) {
                    return config;
                }
            }
        }
        let config = Default::default();
        if let Ok(mut file) = dbg!(OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)) {
            file.write_all(dbg!(to_string_pretty(&config, Default::default())).unwrap().as_bytes()).unwrap_or_default();
        }
        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            save_folder: Opt::Default,
            image_format: ImageFormat::Png,
            enable_startup: false,
        }
    }
}