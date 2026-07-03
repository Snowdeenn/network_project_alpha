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
        offset: $offset:expr, size: $size:expr, bg: $bg:expr, fill: $fill:expr, $(,)?) => {
            {
                use ui::node::{LayoutProps, VisualProps, VisualKind, Anchor};
                use raylib::math::Vector2;

                let bg_id = $ctx.add_node(
                    $parent,
                    LayoutProps::new($anchor, $offset, $size),
                    VisualProps {
                        kind: VisualKind::Rect,
                        color: $bg,
                        opacity: 1.0,
                        visible: true,
                    }
                );
                let fill_id = $ctx.add_node(
                    bg_id,
                    LayoutProps::new(Anchor::TopLeft, Vector2::zero(), $size),
                    VisualProps {
                        kind: VisualKind::Rect,
                        color: $fill,
                        opacity: 1.0,
                        visible: true,
                    }
                );

                (bg_id, fill_id)
            }
    };
    (ctx: $ctx:expr, parent: $parent:expr, anchor: $anchor:expr, 
        offset: $offset:expr, size: $size:expr, bg: $bg:expr, fill_color: $fill_color:expr, shader: $shader:expr, $(,)?) => {
            {

                use ui::node::{LayoutProps, VisualProps, VisualKind, Anchor};
                use raylib::math::Vector2;

                let bg_id = $ctx.add_node(
                    $parent,
                    LayoutProps::new($anchor, $offset, $size),
                    VisualProps {
                        kind: VisualKind::Rect,
                        color: $bg,
                        opacity: 1.0,
                        visible: true,
                    }
                );
                let fill_id = $ctx.add_node(
                    bg_id,
                    LayoutProps::new(Anchor::TopLeft, Vector2::zero(), $size),
                    VisualProps {
                        kind: VisualKind::Shader{ id: $shader },
                        color: $fill_color,
                        opacity: 1.0,
                        visible: true,
                    }
                );

                (bg_id, fill_id)
            }
    }
}
