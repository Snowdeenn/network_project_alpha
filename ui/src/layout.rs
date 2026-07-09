use raylib::math::Vector2;

use crate::node::Anchor;
use crate::node::{UiUnit, UiVec2};

pub fn compute_anchor_pos(
    anchor: Anchor,
    offset: UiVec2,
    node_size: UiVec2,
    parent_pos: Vector2,
    parent_size: Vector2,
    screen_w: f32,
    screen_h: f32,
) -> (Vector2, Vector2)
{
    let ox = resolve_unit(offset.x, screen_w, parent_size.x);
    let oy = resolve_unit(offset.y, screen_h, parent_size.y);
    let nw = resolve_unit(node_size.x, screen_w, parent_size.x);
    let nh = resolve_unit(node_size.y, screen_h, parent_size.y);

    let pos = match anchor {
        Anchor::TopLeft     => Vector2 { x: parent_pos.x + ox,                       y: parent_pos.y + oy },
        Anchor::TopRight    => Vector2 { x: parent_pos.x + parent_size.x - nw - ox,  y: parent_pos.y + oy },
        Anchor::BottomLeft  => Vector2 { x: parent_pos.x + ox,                       y: parent_pos.y + parent_size.y - nh - oy },
        Anchor::BottomRight => Vector2 { x: parent_pos.x + parent_size.x - nw - ox,  y: parent_pos.y + parent_size.y - nh - oy },
        Anchor::Center      => Vector2 { x: parent_pos.x + (parent_size.x - nw) / 2.0 + ox, y: parent_pos.y + (parent_size.y - nh) / 2.0 + oy },
    };

    let size = Vector2 { x: nw, y: nh };
    (pos, size)
}

pub fn resolve_unit(unit: UiUnit, screen: f32, parent: f32) -> f32 {
    match unit {
        UiUnit::Pixels(px)         => px,
        UiUnit::ScreenWidth(ratio) => ratio * screen,
        UiUnit::ScreenHeight(ratio)=> ratio * screen,
        UiUnit::ParentPercent(pct) => pct * parent,
    }
}

