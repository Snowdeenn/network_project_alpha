use raylib::math::Vector2;

use crate::node::Anchor;

pub fn compute_anchor_pos(
    anchor: Anchor,
    offset: Vector2,
    node_size: Vector2,
    parent_pos: Vector2,
    parent_size: Vector2,
) -> Vector2 {
    match anchor {
        Anchor::TopLeft => Vector2 {
            x: parent_pos.x + offset.x,
            y: parent_pos.y + offset.y,
        },
        Anchor::TopRight => Vector2 {
            x: parent_pos.x + parent_size.x - node_size.x - offset.x,
            y: parent_pos.y + offset.y,
        },
        Anchor::BottomLeft => Vector2 {
            x: parent_pos.x + offset.x,
            y: parent_pos.y + parent_size.y - node_size.y - offset.y,
        },
        Anchor::BottomRight => Vector2 {
            x: parent_pos.x + parent_size.x - node_size.x - offset.x,
            y: parent_pos.y + parent_size.y - node_size.y - offset.y,
        },
        Anchor::Center => Vector2 {
            x: parent_pos.x + (parent_size.x - node_size.x) / 2.0 + offset.x,
            y: parent_pos.y + (parent_size.y - node_size.y) / 2.0 + offset.y,
        },
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use raylib::math::Vector2;

    fn setup() -> (Vector2, Vector2, Vector2, Vector2) {
        (
            Vector2 { x: 5.0, y: 10.0 },    // offset
            Vector2 { x: 20.0, y: 30.0 },   // node_size
            Vector2 { x: 10.0, y: 20.0 },   // parent_pos
            Vector2 { x: 100.0, y: 200.0 }, // parent_size
        )
    }

    #[test]
    fn top_left() {
        let (offset, node_size, parent_pos, parent_size) = setup();
        let res = compute_anchor_pos(Anchor::TopLeft, offset, node_size, parent_pos, parent_size);
        
        // x: 10 + 5 = 15
        // y: 20 + 10 = 30
        assert_eq!(res.x, 15.0);
        assert_eq!(res.y, 30.0);
    }

    #[test]
    fn top_right() {
        let (offset, node_size, parent_pos, parent_size) = setup();
        let res = compute_anchor_pos(Anchor::TopRight, offset, node_size, parent_pos, parent_size);
        
        // x: 10 + 100 - 20 - 5 = 85
        // y: 20 + 10 = 30
        assert_eq!(res.x, 85.0);
        assert_eq!(res.y, 30.0);
    }

    #[test]
    fn bottom_left() {
        let (offset, node_size, parent_pos, parent_size) = setup();
        let res = compute_anchor_pos(Anchor::BottomLeft, offset, node_size, parent_pos, parent_size);
        
        // x: 10 + 5 = 15
        // y: 20 + 200 - 30 - 10 = 180
        assert_eq!(res.x, 15.0);
        assert_eq!(res.y, 180.0);
    }

    #[test]
    fn bottom_right() {
        let (offset, node_size, parent_pos, parent_size) = setup();
        let res = compute_anchor_pos(Anchor::BottomRight, offset, node_size, parent_pos, parent_size);
        
        // x: 10 + 100 - 20 - 5 = 85
        // y: 20 + 200 - 30 - 10 = 180
        assert_eq!(res.x, 85.0);
        assert_eq!(res.y, 180.0);
    }

    #[test]
    fn center() {
        let (offset, node_size, parent_pos, parent_size) = setup();
        let res = compute_anchor_pos(Anchor::Center, offset, node_size, parent_pos, parent_size);
        
        // x: 10 + (100 - 20) / 2 + 5 = 10 + 40 + 5 = 55
        // y: 20 + (200 - 30) / 2 + 10 = 20 + 85 + 10 = 115
        assert_eq!(res.x, 55.0);
        assert_eq!(res.y, 115.0);
    }
}
