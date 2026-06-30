use std::collections::{VecDeque, HashSet};

use raylib::{drawing::RaylibDraw, math::Vector2};
use raylib::drawing::RaylibDrawHandle;

use crate::{
    arena::{Arena, NodeId},
    layout::compute_anchor_pos,
    node::{LayoutProps, UiNode, VisualProps, Anchor},
};

pub struct UiContext {
    arena: Arena<UiNode>,
    root: NodeId,
}

impl UiContext {
    pub fn new(screen_w: f32, screen_h: f32) -> Self {
        let mut arena: Arena<UiNode> = Arena::new();
        let root_id = arena.insert(UiNode::new(
            Anchor::Center,
            Vector2::zero(),
            Vector2::new(screen_w, screen_h),
        ));

        if let Some(root_node) = arena.get_mut(root_id) {
            root_node.layout.computed_pos = Vector2::zero();
            root_node.layout.computed_size = Vector2::new(screen_w, screen_h);
        }

        Self {
            arena,
            root: root_id,
        }
    }

    pub fn add_node(&mut self, parent: NodeId, layout: LayoutProps, visual: VisualProps) -> NodeId {
        let mut new_ui_node = UiNode::new(layout.anchor, layout.offset, layout.size);
        new_ui_node.visual = visual;

        let id_new_node = self.arena.insert(new_ui_node);
        if let Some(parent) = self.arena.get_mut(parent) {
            parent.children.push(id_new_node);
        }
        if let Some(new_node) = self.arena.get_mut(id_new_node) {
            new_node.parent = Some(parent);
        }

        id_new_node
    }

    pub fn remove_node(&mut self, id: NodeId) {
        if let Some(node) = self.arena.get(id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.arena.get_mut(parent_id) {
                    parent.children.retain(|c| *c != id);
                }
            }
        }

        self.arena.remove(id);
    }

    fn build_traversal_order(&self) -> Vec<NodeId> {
        let mut order: Vec<NodeId> = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(self.root);

        while !queue.is_empty() {
            if let Some(id) = queue.pop_front() {
                order.push(id);

                if let Some(node) = self.arena.get(id) {
                    for child_id in &node.children {
                        queue.push_back(*child_id);
                    }
                }
            }
        }

        order
    }

    pub fn resolve_layout(&mut self) {
        let order = self.build_traversal_order();

        for id in &order[1..] {
            let Some(parent_id) = self.arena.get(*id).and_then(|n| n.parent) else {
                continue;
            };

            let (parent_pos, parent_size) = if let Some(parent_node) = self.arena.get(parent_id) {
                (
                    parent_node.layout.computed_pos,
                    parent_node.layout.computed_size,
                )
            } else {
                continue;
            };

            let (anchor, offset, size) = if let Some(node) = self.arena.get(*id) {
                (node.layout.anchor, node.layout.offset, node.layout.size)
            } else {
                continue;
            };

            let new_pos = compute_anchor_pos(anchor, offset, size, parent_pos, parent_size);
            if let Some(node) = self.arena.get_mut(*id) {
                node.layout.computed_pos = new_pos;
                node.layout.computed_size = size;
            }
        }
    }

    pub fn render(&self, d: &mut RaylibDrawHandle) {
    let order = self.build_traversal_order();
    let mut hidden: HashSet<NodeId> = HashSet::new();

    for id in order {
        if let Some(node) = self.arena.get(id) {

            let parent_hidden = node.parent
                .map(|p| hidden.contains(&p))
                .unwrap_or(false);

            if !node.visual.visible || parent_hidden {
                hidden.insert(id);
                continue;
            }

            let color = node.visual.color.alpha(node.visual.opacity);
            d.draw_rectangle_v(node.layout.computed_pos, node.layout.computed_size, color);
        }
    }
}
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::node::Anchor;

    fn setup() -> (UiContext, LayoutProps, VisualProps) {
        (
            UiContext::new(1920.0, 1080.0),
            LayoutProps::new(
                Anchor::Center,
                Vector2::new(20.0, 20.0),
                Vector2::new(180.0, 40.0),
            ),
            VisualProps::default(),
        )
    }

    #[test]
    fn test_regression() {
        let (mut ui_ctx, layout, visual) = setup();
        let id_child = ui_ctx.add_node(ui_ctx.root, layout, visual);

        ui_ctx.remove_node(id_child);
        let is_empty = ui_ctx.arena.get(ui_ctx.root).unwrap().children.is_empty();
        assert!(is_empty);
    }

    #[test]
    fn test_taversal_order() {
        let (mut ui_ctx, layout1, visual1) = setup();
        let (_, layout2, visual2) = setup();
        let (_, layout3, visual3) = setup();

        ui_ctx.add_node(ui_ctx.root, layout1, visual1);
        ui_ctx.add_node(ui_ctx.root, layout2, visual2);
        ui_ctx.add_node(ui_ctx.root, layout3, visual3);

        let order = ui_ctx.build_traversal_order();

        assert_eq!(order[0], ui_ctx.root);
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn test_pipline() {
        let mut ui_ctx = UiContext::new(1920.0, 1080.0);
        let layout = LayoutProps::new(
            Anchor::TopRight,
            Vector2::new(20.0, 20.0),
            Vector2::new(180.0, 40.0),
        );
        let node_id = ui_ctx.add_node(ui_ctx.root, layout, VisualProps::default());
        ui_ctx.resolve_layout();

        let node = ui_ctx.arena.get(node_id).expect("node devrait exister");
        let expected = Vector2::new(1720.0, 20.0);
        assert_eq!(node.layout.computed_pos, expected);
    }
}
