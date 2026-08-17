use std::ops::{Add, Div, Mul, Sub};
use utils::colors;
use utils::math::Vec2;

use crate::NodeId;
use crate::input::Interact;
use utils::ids::{TextureId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    BottomLeft,
    TopRight,
    BottomRight,
    Center,
}

pub struct LayoutProps {
    pub anchor: Anchor,
    pub offset: UiVec2,
    pub size: UiVec2,
    pub(crate) computed_pos: Vec2,
    pub(crate) computed_size: Vec2,
}

impl LayoutProps {
    pub fn new(anchor: Anchor, offset: UiVec2, size: UiVec2) -> Self {
        LayoutProps {
            anchor,
            offset,
            size,
            computed_pos: Vec2::zero(),
            computed_size: Vec2::zero(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VisualKind {
    Rect,
    Texture {
        id: TextureId,
    },
    Material {
        material_id: utils::ids::MaterialId,
        texture_id: Option<TextureId>,
        uniform_data: Vec<u8>,
    },
    NinePatch {
        id: TextureId,
        margins: prism::NinePatchMargins,
    },
    Text {
        content: String,
        font_size: f32,
    },
    None,
}

pub struct VisualProps {
    pub kind: VisualKind,
    pub color: colors::Color,
    pub visible: bool,
    pub opacity: f32,
}

impl Default for VisualProps {
    fn default() -> Self {
        VisualProps {
            kind: VisualKind::Rect,
            color: colors::Color::WHITE,
            visible: true,
            opacity: 1.0,
        }
    }
}

pub struct DirtyFlags {
    pub layout_dirty: bool,
    pub visual_dirty: bool,
}

impl Default for DirtyFlags {
    fn default() -> Self {
        DirtyFlags {
            layout_dirty: true,
            visual_dirty: true,
        }
    }
}

pub struct UiNode {
    pub layout: LayoutProps,
    pub visual: VisualProps,
    pub interact: Option<Interact>,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub dirty: DirtyFlags,
}

impl UiNode {
    pub fn new(anchor: Anchor, offset: UiVec2, size: UiVec2) -> Self {
        Self {
            layout: LayoutProps::new(anchor, offset, size),
            visual: VisualProps::default(),
            children: Vec::new(),
            parent: None,
            dirty: DirtyFlags::default(),
            interact: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UiUnit {
    Pixels(f32),        // valeur fixe en pixels
    ScreenWidth(f32),   // ratio de la largeur de l'écran  (0.1 = 10% de screen_w)
    ScreenHeight(f32),  // ratio de la hauteur de l'écran  (0.1 = 10% de screen_h)
    ParentPercent(f32), // ratio de la dimension du parent (0.5 = 50% du parent)
}

impl Mul<f32> for UiUnit {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            UiUnit::Pixels(x) => UiUnit::Pixels(x * rhs),
            UiUnit::ScreenWidth(x) => UiUnit::ScreenWidth(x * rhs),
            UiUnit::ScreenHeight(x) => UiUnit::ScreenHeight(x * rhs),
            UiUnit::ParentPercent(x) => UiUnit::ParentPercent(x * rhs),
        }
    }
}

impl Sub<UiUnit> for f32 {
    type Output = UiUnit;

    fn sub(self, rhs: UiUnit) -> Self::Output {
        match rhs {
            UiUnit::Pixels(x) => UiUnit::Pixels(self - x),
            UiUnit::ScreenWidth(x) => UiUnit::ScreenWidth(self - x),
            UiUnit::ScreenHeight(x) => UiUnit::ScreenHeight(self - x),
            UiUnit::ParentPercent(x) => UiUnit::ParentPercent(self - x),
        }
    }
}

impl Div<f32> for UiUnit {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        match self {
            UiUnit::Pixels(x) => UiUnit::Pixels(x / rhs),
            UiUnit::ScreenWidth(x) => UiUnit::ScreenWidth(x / rhs),
            UiUnit::ScreenHeight(x) => UiUnit::ScreenHeight(x / rhs),
            UiUnit::ParentPercent(x) => UiUnit::ParentPercent(x / rhs),
        }
    }
}

impl Add<UiUnit> for UiUnit {
    type Output = Self;

    fn add(self, rhs: UiUnit) -> Self::Output {
        match (self, rhs) {
            (UiUnit::Pixels(a), UiUnit::Pixels(b)) => UiUnit::Pixels(a + b),
            (UiUnit::ScreenWidth(a), UiUnit::ScreenWidth(b)) => UiUnit::ScreenWidth(a + b),
            (UiUnit::ScreenHeight(a), UiUnit::ScreenHeight(b)) => UiUnit::ScreenHeight(a + b),
            (UiUnit::ParentPercent(a), UiUnit::ParentPercent(b)) => UiUnit::ParentPercent(a + b),

            _ => panic!(
                "Opération Add impossible entre des UiUnit de types différents directement. Attendez la résolution du Layout."
            ),
        }
    }
}

impl Sub<UiUnit> for UiUnit {
    type Output = Self;

    fn sub(self, rhs: UiUnit) -> Self::Output {
        match (self, rhs) {
            (UiUnit::Pixels(a), UiUnit::Pixels(b)) => UiUnit::Pixels(a - b),
            (UiUnit::ScreenWidth(a), UiUnit::ScreenWidth(b)) => UiUnit::ScreenWidth(a - b),
            (UiUnit::ScreenHeight(a), UiUnit::ScreenHeight(b)) => UiUnit::ScreenHeight(a - b),
            (UiUnit::ParentPercent(a), UiUnit::ParentPercent(b)) => UiUnit::ParentPercent(a - b),

            _ => panic!(
                "Opération Sub impossible entre des UiUnit de types différents directement. Attendez la résolution du Layout."
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UiVec2 {
    pub x: UiUnit,
    pub y: UiUnit,
}

impl UiVec2 {
    pub fn new(x: UiUnit, y: UiUnit) -> Self {
        Self { x, y }
    }

    pub fn pixels(x: f32, y: f32) -> Self {
        Self {
            x: UiUnit::Pixels(x),
            y: UiUnit::Pixels(y),
        }
    }

    pub fn screen(x: f32, y: f32) -> Self {
        Self {
            x: UiUnit::ScreenWidth(x),
            y: UiUnit::ScreenHeight(y),
        }
    }
}
