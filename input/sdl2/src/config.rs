use sdl2::keyboard::Scancode;
use sdl2::mouse::MouseButton;

pub struct InputConfigSdl {
    pub(crate) key_right: Scancode,
    pub(crate) key_left: Scancode,
    pub(crate) key_up: Scancode,
    pub(crate) key_down: Scancode,
    pub(crate) key_strafeleft: Scancode,
    pub(crate) key_straferight: Scancode,
    pub(crate) key_fire: Scancode,
    pub(crate) key_use: Scancode,
    pub(crate) key_strafe: Scancode,
    pub(crate) key_speed: Scancode,
    pub(crate) mousebfire: MouseButton,
    pub(crate) mousebstrafe: MouseButton,
    pub(crate) mousebforward: MouseButton,
}

impl From<&InputConfig> for InputConfigSdl {
    fn from(i: &InputConfig) -> Self {
        Self {
            key_right: Scancode::from_i32(i.key_right).unwrap(),
            key_left: Scancode::from_i32(i.key_left).unwrap(),
            key_up: Scancode::from_i32(i.key_up).unwrap(),
            key_down: Scancode::from_i32(i.key_down).unwrap(),
            key_strafeleft: Scancode::from_i32(i.key_strafeleft).unwrap(),
            key_straferight: Scancode::from_i32(i.key_straferight).unwrap(),
            key_fire: Scancode::from_i32(i.key_fire).unwrap(),
            key_use: Scancode::from_i32(i.key_use).unwrap(),
            key_strafe: Scancode::from_i32(i.key_strafe).unwrap(),
            key_speed: Scancode::from_i32(i.key_speed).unwrap(),
            mousebfire: MouseButton::from_ll(i.mousebfire),
            mousebstrafe: MouseButton::from_ll(i.mousebstrafe),
            mousebforward: MouseButton::from_ll(i.mousebforward),
        }
    }
}
