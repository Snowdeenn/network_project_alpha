use std::collections::{HashSet, VecDeque};

use raylib::math::Vector2;

use crate::draw::{DrawCommand, DrawCommandBuffer};
use crate::event::UIEvent;
use crate::node::VisualKind;
use crate::tween::TweenEngine;
use crate::{
    arena::{Arena, NodeId},
    layout::compute_anchor_pos,
    node::{Anchor, LayoutProps, UiNode, VisualProps},
};

pub struct UiContext {
    arena: Arena<UiNode>,
    pub root: NodeId,
    events: VecDeque<UIEvent>,
    pub tween: TweenEngine,
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
            root_node.visual.visible = false;
        }

        Self {
            arena,
            root: root_id,
            events: VecDeque::new(),
            tween: TweenEngine::default(),
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

    fn resolve_layout(&mut self) {
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

    pub fn collect(&self, buf: &mut DrawCommandBuffer) {
        let order = self.build_traversal_order();
        let mut hidden: HashSet<NodeId> = HashSet::new();

        for (depth, id) in order[1..].iter().enumerate() {
            if let Some(node) = self.arena.get(*id) {
                let parent_hidden = node.parent.map(|p| hidden.contains(&p)).unwrap_or(false);

                if !node.visual.visible || parent_hidden {
                    hidden.insert(*id);
                    continue;
                }

                let color = node.visual.color.alpha(node.visual.opacity);
                match &node.visual.kind {
                    VisualKind::Rect => {
                        buf.push(DrawCommand::Rect {
                            pos: node.layout.computed_pos,
                            size: node.layout.computed_size,
                            color,
                            layer: depth as u8,
                        });
                    }
                    VisualKind::Texture { id } => {
                        buf.push(DrawCommand::Texture {
                            texture_id: *id,
                            pos: node.layout.computed_pos,
                            size: node.layout.computed_size,
                            tint: color,
                            layer: depth as u8,
                        });
                    }
                    VisualKind::Shader { id } => {
                        buf.push(DrawCommand::Shader {
                            shader_id: *id,
                            pos: node.layout.computed_pos,
                            size: node.layout.computed_size,
                            color,
                            layer: depth as u8,
                        });
                    }
                    VisualKind::ShaderTexture {
                        shader_id,
                        texture_id,
                    } => {
                        buf.push(DrawCommand::ShaderTexture {
                            shader_id: *shader_id,
                            texture_id: *texture_id,
                            pos: node.layout.computed_pos,
                            size: node.layout.computed_size,
                            tint: color,
                            layer: depth as u8,
                        });
                    }
                    VisualKind::NinePatch { id, margins } => {
                        buf.push(DrawCommand::NinePatch {
                            texture_id: *id,
                            pos: node.layout.computed_pos,
                            size: node.layout.computed_size,
                            margins: *margins,
                            tint: color,
                            layer: depth as u8,
                        });
                    }
                    VisualKind::Text { content, font_size } => {
                        buf.push(DrawCommand::Text {
                            text: content.clone(),
                            pos: node.layout.computed_pos,
                            font_size: *font_size,
                            color,
                            layer: depth as u8,
                        });
                    }
                }
            }
        }
    }

    pub fn send_event(&mut self, event: UIEvent) {
        self.events.push_back(event);
    }

    fn process_event(&mut self) {
        while let Some(event) = self.events.pop_front() {
            match event {
                UIEvent::SetColor { target, color } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        node.visual.color = color;
                        node.dirty.visual_dirty = true;
                    }
                }
                UIEvent::SetOpacity { target, opacity } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        node.visual.opacity = opacity;
                        node.dirty.visual_dirty = true;
                    }
                }
                UIEvent::SetVisible { target, visible } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        node.visual.visible = visible;
                        node.dirty.visual_dirty = true;
                    }
                }
                UIEvent::SetPosition { target, offset } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        node.layout.offset = offset;
                        node.dirty.layout_dirty = true;
                    }
                }
                UIEvent::SetSize { target, size } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        node.layout.size = size;
                        node.dirty.layout_dirty = true;
                    }
                }
                UIEvent::SetTexture { target, id } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        node.visual.kind = VisualKind::Texture { id };
                        node.dirty.visual_dirty = true;
                    }
                }
                UIEvent::SetShader { target, id } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        node.visual.kind = VisualKind::Shader { id };
                        node.dirty.visual_dirty = true;
                    }
                }
                UIEvent::SetText { target, content } => {
                    if let Some(node) = self.arena.get_mut(target) {
                        if let VisualKind::Text { content: c, .. } = &mut node.visual.kind {
                            *c = content;
                            node.dirty.visual_dirty = true;
                        }
                    }
                }
            }
        }
    }

    pub fn update(&mut self, dt: f32) {
        let mut tween = std::mem::take(&mut self.tween);
        tween.update(dt);

        for event in tween.drain_events() {
            self.send_event(event);
        }
        self.tween = tween;

        self.process_event();

        let need_resolve_layout = self.arena.iter().any(|node| node.dirty.layout_dirty);
        if need_resolve_layout {
            self.resolve_layout();
        }
        for node in self.arena.iter_mut() {
            node.dirty.layout_dirty = false;
            node.dirty.visual_dirty = false;
        }
    }
}

#[cfg(test)]
mod test {
    // dans Arena<T>

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
