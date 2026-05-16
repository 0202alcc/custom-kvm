use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum KvmEvent {
    MouseMove { dx: i32, dy: i32 },
    MouseAbsMove { x: i32, y: i32 }, // Ensure this line exists here!
    MouseButton { button: u8, is_down: bool },
    Key { keycode: u16, is_down: bool },
    MouseScroll { delta: i32 },
}