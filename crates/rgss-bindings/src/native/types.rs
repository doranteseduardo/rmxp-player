#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RectData {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl RectData {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorData {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl ColorData {
    pub fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ToneData {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub gray: f32,
}

impl ToneData {
    pub fn new(red: f32, green: f32, blue: f32, gray: f32) -> Self {
        Self {
            red,
            green,
            blue,
            gray,
        }
    }
}
