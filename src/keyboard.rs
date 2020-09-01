use user32::{GetKeyState};
use winapi::{
    um::winuser::{VK_SNAPSHOT, VK_MENU, /*VK_ESCAPE,*/},
};

pub struct KeyState {
    pub vk_snapshot: VirtualKey,
    pub vk_menu: VirtualKey,
    //vk_escape: VirtualKey,
}

#[derive(PartialEq, Eq)]
pub enum KeyEvent {
    Up,
    Fire,
    Down
}

pub struct VirtualKey(u32);

impl VirtualKey {
    pub fn is_down(&self) -> bool {
        self.0 & 0x8000 != 0
    }
}

pub fn retrieve_keys() -> KeyState {
    unsafe {
        KeyState {
            vk_snapshot: VirtualKey(GetKeyState(VK_SNAPSHOT) as u32),
            vk_menu: VirtualKey(GetKeyState(VK_MENU) as u32),
            //vk_escape: VirtualKey(GetKeyState(VK_ESCAPE) as u32),
        }
    }
}