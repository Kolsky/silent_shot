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
    pub save_folder: Opt<String>,
    pub image_format: ImageFormat,
    pub enable_startup: bool,
}

const CONFIG_HELP : &str =
"//(
//    save_folder: [Default]|Custom(\"C:/path/to/your/dir\"),
//    Sets folder for screenshots. Default is \"%USERPROFILE%/Images/Screenshots\".
//
//    image_format: [Png]|Tga|Both,
//    Saving .tga does not require separate thread for converting it, .png files are smaller.
//
//    enable_startup: true|[false],
//    Chooses whether the app should run automatically at Windows startup.
//)

";

impl Config {
    pub fn open_or_create_default<T: AsRef<Path>>(path: T) -> Self {
        if let Ok(mut file) = dbg!(File::open(&path)) {
            let mut contents = String::new();
            if let Ok(_) = file.read_to_string(&mut contents) {
                if let Ok(config) = from_str(&contents) {
                    return config;
                }
            }
        }
        let config = Default::default();
        if let Ok(mut file) = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path) {
            file.write_all(CONFIG_HELP.as_bytes()).unwrap_or_default();    
            file.write_all(to_string_pretty(&config, Default::default()).unwrap().as_bytes()).unwrap_or_default();
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