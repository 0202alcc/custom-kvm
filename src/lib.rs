use serde::{Serialize, Deserialize};

pub mod error;
pub use error::{Error, Result};

pub mod logging;

pub mod config;

pub mod keycodes;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum KvmEvent {
    MouseMove { dx: i32, dy: i32 },
    MouseAbsMove { x: i32, y: i32 }, // Ensure this line exists here!
    MouseButton { button: u8, is_down: bool },
    Key { keycode: u16, is_down: bool },
    MouseScroll { delta: i32 },
}