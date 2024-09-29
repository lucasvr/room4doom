#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Keycode {}

impl Keycode {
    pub fn from_scancode(_scancode: Scancode) -> Option<Keycode> {
        None
    }

    pub fn into_i32(&self) -> i32 {
        0
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum Scancode {
    Left,
    Right,
    Up,
    Down,
    W,
    S,
    A,
    D,
    RCtrl,
    Space,
    RAlt,
    LShift,
    Return,
    Escape,
    Backspace,
    F1,
    F2,
    F3,
    F6,
    F9,
    Pause,
}

impl ToString for Scancode {
    fn to_string(&self) -> String {
        match self {
            Scancode::Left => "Left".to_string(),
            Scancode::Right => "Right".to_string(),
            Scancode::Up => "Up".to_string(),
            Scancode::Down => "Down".to_string(),
            Scancode::W => "W".to_string(),
            Scancode::S => "S".to_string(),
            Scancode::A => "A".to_string(),
            Scancode::D => "D".to_string(),
            Scancode::RCtrl => "RCtrl".to_string(),
            Scancode::Space => "Space".to_string(),
            Scancode::RAlt => "RAlt".to_string(),
            Scancode::LShift => "LShift".to_string(),
            Scancode::Return => "Return".to_string(),
            Scancode::Escape => "Escape".to_string(),
            Scancode::Backspace => "Backspace".to_string(),
            Scancode::F1 => "F1".to_string(),
            Scancode::F2 => "F2".to_string(),
            Scancode::F3 => "F3".to_string(),
            Scancode::F6 => "F6".to_string(),
            Scancode::F9 => "F9".to_string(),
            Scancode::Pause => "Pause".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum MouseButton {
    Left,
    Right,
    Middle
}

pub struct Input {}
impl Input {
    pub fn update(&mut self, mut _key_once_callback: impl FnMut(Scancode) -> bool) -> bool {
        false
    }
}
