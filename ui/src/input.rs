use raylib::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InteractState {
    Normal,
    Hover,
    Pressed,
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonStyle {
    pub normal  : Color,
    pub hover   : Color,
    pub pressed : Color,
}

#[derive(Debug, Clone)]
pub struct Interact {
    pub state : InteractState,
    pub style : ButtonStyle,
}


