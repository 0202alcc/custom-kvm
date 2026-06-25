use serde::{Serialize, Deserialize};

pub mod error;
pub use error::{Error, Result};

pub mod logging;

pub mod config;

pub mod keycodes;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum DisplayOrientation {
    Normal,
    Left,
    Right,
    Inverted,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DisplayInfo {
    pub id: u32,
    pub orientation: DisplayOrientation,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum KvmEvent {
    MouseMove { dx: i32, dy: i32 },
    MouseAbsMove { x: i32, y: i32 }, // Ensure this line exists here!
    MouseButton { button: u8, is_down: bool },
    Key { keycode: u16, is_down: bool },
    MouseScroll { delta: i32 },
    Heartbeat,
    DisplayReport { displays: Vec<DisplayInfo> },
    LogMessage { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDetectionResult {
    None,
    TransitionToMac,
    TransitionToLinux,
}

pub struct EdgeDetector {
    virtual_x: i32,
    virtual_y: i32,
    is_controlling_mac: bool,
    screen_width: i32,
    screen_height: i32,
}

impl EdgeDetector {
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        Self {
            virtual_x: screen_width / 2,
            virtual_y: screen_height / 2,
            is_controlling_mac: false,
            screen_width,
            screen_height,
        }
    }

    pub fn update(&mut self, dx: i32, dy: i32) -> EdgeDetectionResult {
        self.virtual_x += dx;
        self.virtual_y += dy;
        self.virtual_y = self.virtual_y.clamp(0, self.screen_height);

        if !self.is_controlling_mac {
            if self.virtual_x >= self.screen_width - 1 {
                self.is_controlling_mac = true;
                EdgeDetectionResult::TransitionToMac
            } else {
                self.virtual_x = self.virtual_x.clamp(0, self.screen_width);
                EdgeDetectionResult::None
            }
        } else {
            if self.virtual_x < 0 {
                self.is_controlling_mac = false;
                self.virtual_x = self.screen_width - 10;
                EdgeDetectionResult::TransitionToLinux
            } else {
                EdgeDetectionResult::None
            }
        }
    }

    pub fn virtual_x(&self) -> i32 {
        self.virtual_x
    }

    pub fn virtual_y(&self) -> i32 {
        self.virtual_y
    }

    pub fn is_controlling_mac(&self) -> bool {
        self.is_controlling_mac
    }
}
