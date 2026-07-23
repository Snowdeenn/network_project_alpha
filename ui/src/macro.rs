#[macro_export]
macro_rules! text_label {
    (
        ctx: $ctx:expr,
        parent: $parent:expr,
        anchor: $anchor:expr,
        offset: $offset:expr,
        size: $size:expr,
        content: $content:expr,
        font_size: $font_size:expr,
        color: $color:expr
        $(,)?
    ) => {{
        use ui::node::{LayoutProps, VisualKind, VisualProps};
        $ctx.add_node(
            $parent,
            LayoutProps::new($anchor, $offset, $size),
            VisualProps {
                kind: VisualKind::Text {
                    content: $content.to_string(),
                    font_size: $font_size,
                },
                color: $color,
                opacity: 1.0,
                visible: true,
            },
        )
    }};
    (ctx: $ctx:expr, parent: $parent:expr, layout: $layout:expr, visual: $visual:expr) => {
        $ctx.add_node($parent, $layout, $visual)
    };
}

#[macro_export]
macro_rules! progress_bar {
    (ctx: $ctx:expr, parent: $parent:expr, anchor: $anchor:expr, 
        offset: $offset:expr, size: $size:expr, bg: $bg:expr, fill: $fill:expr, $(,)?) => {{
        use raylib::math::Vector2;
        use ui::node::{Anchor, LayoutProps, VisualKind, VisualProps};

        let bg_id = $ctx.add_node(
            $parent,
            LayoutProps::new($anchor, $offset, $size),
            VisualProps {
                kind: VisualKind::Rect,
                color: $bg,
                opacity: 1.0,
                visible: true,
            },
        );
        let fill_id = $ctx.add_node(
            bg_id,
            LayoutProps::new(
                Anchor::TopLeft,
                UiVec2::pixels(0.0, 0.0),
                UiVec2::new(UiUnit::ParentPercent(1.0), UiUnit::ParentPercent(1.0)),
            ),
            VisualProps {
                kind: VisualKind::Rect,
                color: $fill,
                opacity: 1.0,
                visible: true,
            },
        );

        (bg_id, fill_id)
    }};
    (ctx: $ctx:expr, parent: $parent:expr, anchor: $anchor:expr, 
        offset: $offset:expr, size: $size:expr, bg: $bg:expr, fill_color: $fill_color:expr, shader: $shader:expr, $(,)?) => {{
        use raylib::math::Vector2;
        use ui::node::{Anchor, LayoutProps, UiVec2, VisualKind, VisualProps};

        let bg_id = $ctx.add_node(
            $parent,
            LayoutProps::new($anchor, $offset, $size),
            VisualProps {
                kind: VisualKind::Rect,
                color: $bg,
                opacity: 1.0,
                visible: true,
            },
        );
        let fill_id = $ctx.add_node(
            bg_id,
            LayoutProps::new(Anchor::TopLeft, UiVec2::pixels(0.0, 0.0), $size),
            VisualProps {
                kind: VisualKind::Shader { id: $shader },
                color: $fill_color,
                opacity: 1.0,
                visible: true,
            },
        );

        (bg_id, fill_id)
    }};
}

#[macro_export]
macro_rules! bouton {
    (ctx: $ctx:expr, parent: $parent:expr, anchor: $anchor:expr, offset: $offset:expr, 
        size: $size:expr, normal: $normal:expr, hover: $hover:expr, pressed: $pressed:expr, $(,)?) => {{
        use ui::input::{ButtonStyle, Interact, InteractState};
        use ui::node::{Anchor, LayoutProps, VisualKind, VisualProps};

        let _id = $ctx.add_node(
            $parent,
            LayoutProps::new($anchor, $offset, $size),
            VisualProps {
                kind: VisualKind::Rect,
                color: $normal,
                opacity: 1.0,
                visible: true,
            },
        );
        $ctx.set_interact(
            _id,
            Interact {
                state: InteractState::Normal,
                style: ButtonStyle {
                    normal: $normal,
                    hover: $hover,
                    pressed: $pressed,
                },
            },
        );
        _id
    }};
}
