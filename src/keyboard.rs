use user32::{GetKeyState};
use winapi::{
    um::winuser::{VK_SNAPSHOT, VK_MENU, /*VK_ESCAPE,*/},
};

pub struct KeyState {
    pub vk_snapshot: u32,
    pub vk_menu: u32,
    //vk_escape: u32,
}

#[derive(PartialEq, Eq)]
pub enum KeyEvent {
    Up,
    Fire,
    Down
}

pub trait VirtualKey {
    fn is_down(&self) -> bool;
}

impl VirtualKey for u32 {
    fn is_down(&self) -> bool {
        self & 0x8000 != 0
    }
}

pub fn retrieve_keys() -> KeyState {
    unsafe {
        KeyState {
            vk_snapshot: GetKeyState(VK_SNAPSHOT) as u32,
            vk_menu: GetKeyState(VK_MENU) as u32,
            //vk_escape: GetKeyState(VK_ESCAPE) as u32,
        }
    }
}